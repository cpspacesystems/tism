//! # TOM Interface for Shared Memory
//!
//! [`tism`] is a safe wrapper over LibC shared memory functionality and the inter-process
//! communication framework for CPSS TOM's flight processes.
//!
//! [`tism`] works with two kinds of processes:
//!  - Publishers, which create shared memory allocations and populate them with data, and work with
//!    [`tism`] using the [`tism::create`] function and [`OwnedSharedMemory`] type.
//!  - Consumers, which open existing shared memory allocation to read data off of them, and work
//!    with the [`tism::open`] function and [`BorrowedSharedMemory`] type.
//!
//! There may only be one publisher per shared memory allocation, but [`tism`] allows for there to
//! be as many consumers as desired. Publishers are prevented from writing to memory when it is
//! being read by a consumer, but consumers may read the memory at the same time as any other
//! consumer so long as the publisher is not writing to it.
//!
//! When naming your allocations do not include a slash at the beginning, ommit it and [`tism`] will
//! insert it where needed, as the behavior differs between platforms.
//!
//! # Examples
//!
//! Note: `cargo` runs doc tests in parallel, as a consequence of this if I want to run tests on the
//! examples provided here they need unique names for allocations. Don't worry about this in your
//! own code, so long as each allocation corresponds to one data type and has only one publisher
//! you'll be fine - [`tism`] cleans up after itself and, in the case your program exited in a less
//! than grateful manner, new attempts to create shared memory allocations will clean up after the
//! earlier process.
//!
//! ## Publisher
//!
//! This is the recommended way of working with [`tism`] for publisher processes, it requires that
//! `T` be nothing but [`Sized`] (for interoperability with C or Python you must make your type with
//! `#[repr(C)]`).
//!
//! ```
//! // For this version we won't need `Clone` or `Copy`
//! #[repr(C)]
//! #[derive(PartialEq, Debug)]
//! struct MyData {
//!     field_1: i32,
//!     field_2: f64,
//! }
//!
//! let init_data = MyData { field_1: 37, field_2: 3.1415 };
//! let mut my_shm = tism::create("shm_owned_example", init_data).unwrap();
//!
//! // by pattern matching on the lock result, we confine our lock to the
//! // scope of this if statement
//! if let Ok(mut lock) = my_shm.write_lock() {
//!     // We can get a reference to our shared data and access individual
//!     // fields easily and without copying.
//!     let x: &MyData = lock.as_ref();
//!     assert_eq!(x.field_1, 37);
//!
//!     // Since we have a write lock, we can also mutate fields.
//!     lock.as_mut().field_2 = 3f64;
//!
//!     // If we want we can also overwrite or read the entire struct.
//!     assert_eq!(lock.as_ref(), &MyData { field_1: 37, field_2: 3f64 });
//! }  // When `lock` drops it unlocks!
//!
//! ```
//!
//! This next way is simpler, but takes ownership of the given value of `T`. If you were to make
//! many changes in sequence it would rapidly lock and unlock the shared memory. If you make only
//! one write then this method is perfect, for more complicated uses the method outlined in the
//! previous example would be superior.
//!
//! ```
//! // For this version we do need `Clone`
//! #[repr(C)]
//! #[derive(Clone, Copy, PartialEq, Debug)]
//! struct MyData {
//!     field_1: i32,
//!     field_2: f64,
//! }
//!
//! let init_data = MyData { field_1: 37, field_2: 3.1415 };
//! let mut my_shm = tism::create("shm_write_onestep_example", init_data).unwrap();
//!
//! // locking happens internally, `write` clones the value you give it
//! my_shm.write(MyData { field_1: 0, field_2: 0f64 }).unwrap();
//!
//! // the `read` function clones the contents of the shared memory and gives
//! // you the cloned value
//! let x = my_shm.read().unwrap();
//! assert_eq!(x, MyData { field_1: 0, field_2: 0f64 });
//! ```
//!
//! ## Consumer
//!
//! This is the recommended way for consumers to use shared memory, it allows direct, _zero-copy_
//! access to the shared memory, allowing you to more efficiently extract particular fields.
//!
//! ```
//! #[repr(C)]
//! #[derive(PartialEq, Debug)]
//! struct MyData {
//!     field_1: i32,
//!     field_2: f64,
//! }
//!
//! // Create shared memory so we can open it later :)
//! let init_data = MyData { field_1: 37, field_2: 3.1415 };
//! let my_shm_owner = tism::create("shm_zerocopy_consumer_example", init_data).unwrap();
//!
//! // We can also use the `tism::open` function if we want our open to fail if
//! // the allocation does not yet exist, this call will wait until the
//! // allocation is available.
//! let mut my_shm = tism::wait_and_open::<MyData>("shm_zerocopy_consumer_example").unwrap();
//!
//! if let Ok(lock) = my_shm.read_lock() {
//!     let x: &MyData = lock.as_ref();
//!     assert_eq!(lock.as_ref().field_2, 3.1415);
//! }
//! ```
//!
//! If `T` is [`Clone`] you can also use this simpler method, though it is less efficient:
//!
//! ```
//! #[repr(C)]
//! #[derive(Clone, Copy, PartialEq, Debug)]
//! struct MyData {
//!     field_1: i32,
//!     field_2: f64,
//! }
//!
//! let init_data = MyData { field_1: 37, field_2: 3.1415 };
//! let my_shm_owner = tism::create("shm_clone_consumer_example", init_data).unwrap();
//!
//! let mut my_shm = tism::wait_and_open::<MyData>("shm_clone_consumer_example").unwrap();
//!
//! let x = my_shm.read().unwrap();
//! assert_eq!(x, MyData { field_1: 37, field_2: 3.1415 });
//! ```
//!
//! [`tism`]: crate
//! [`tism::create`]: create
//! [`tism::open`]: open
//! [`OwnedSharedMemory`]: OwnedSharedMemory
//! [`BorrowedSharedMemory`]: BorrowedSharedMemory
//! [`Sized`]: Sized

const MAJOR_VERSION: u8 = 1;
const MINOR_VERSION: u8 = 0;
const PATCH_VERSION: u16 = 0;

pub mod dynamic;
pub mod lazy;

#[cfg(test)]
mod tests;

use libc::{
    self, CLOCK_MONOTONIC, ENOENT, O_CREAT, O_EXCL, O_RDWR, O_TRUNC, S_IRGRP, S_IROTH, S_IRUSR,
    S_IWGRP, S_IWUSR, close, ftruncate, munmap, pthread_rwlock_init, pthread_rwlock_rdlock,
    pthread_rwlock_t, pthread_rwlock_unlock, pthread_rwlock_wrlock, shm_open,
};
use std::{
    io,
    path::Path,
    ptr,
    sync::atomic::{self, AtomicU64},
    time::{Duration, SystemTime},
};

/// The overhead of a tism allocation.
const TISM_OVERHEAD: usize = size_of::<Allocation<()>>();

/// Create a new shared memory allocaton, which you will own, initialized to the given value of `T`.
/// If a shared memory allocation by the given name already exists this is not considered an error,
/// since its possible that we are reaquiring an allocation which we failed to clean up earlier, so
/// care should be taken not to have two processes call [`tism::create`] with the same name so long
/// as their usage overlaps.
///
/// See docs on [`tism::OwnedSharedMemory`] for info on how resources are freed. Do not including a
/// leading "/" in the given name.
///
/// [`tism::create`]: create
/// [`tism::OwnedSharedMemory`]: OwnedSharedMemory
pub fn create<T>(name: impl AsRef<Path>, init: T) -> io::Result<OwnedSharedMemory<T>> {
    let mut shm = unsafe { SharedMemory::create(name) }?;

    unsafe {
        shm.write_lock()?;
        (*shm.allocation).data = init;
        shm.unlock()?;
    }

    Ok(OwnedSharedMemory(shm))
}

/// Open an existing already allocated shared memory allocation. This function returns an [`Err`] if
/// the allocation does not already exist.
///
/// See docs on [`tism::BorrowedSharedMemory`] for info on how resources are freed.
///
/// Within the [`tism`] API this value will always be initialized, but if for some reason [`tism`]
/// is being used to interact with allocations by other libraries it is possible that the allocation
/// is not initialized, [`tism`] cannot check this.
///
/// [`tism::BorrowedSharedMemory`]: BorrowedSharedMemory
/// [`tism`]: crate
/// [`Err`]: Result::Err
pub fn open<T>(name: impl AsRef<Path>) -> io::Result<BorrowedSharedMemory<T>> {
    let shm = SharedMemory::open(name)?;
    Ok(BorrowedSharedMemory(shm))
}

/// Continually attempt to open a shared memory allocation, retrying if the allocation does not
/// exist. All other kinds of errors are still propogated. See [`tism::open`] for more info.
///
/// [`tism::open`]: open
pub fn wait_and_open<T>(name: impl AsRef<Path>) -> io::Result<BorrowedSharedMemory<T>> {
    loop {
        match SharedMemory::open(name.as_ref()) {
            Ok(shm) => return Ok(BorrowedSharedMemory(shm)),

            Err(io_err) => match io_err.kind() {
                // We only accept "not found" errors
                io::ErrorKind::NotFound => continue,

                // Propogate anything else
                _ => return Err(io_err),
            },
        }
    }
}

/// Represents an owned (i.e. created) shared memory allocaton. Only the process which holds the
/// [`tism::OwnedSharedMemory`] instance is allowed to write to the memory. This process may be
/// refered to as the "publisher" in some parts of [`tism`]'s documentation.
///
/// When a [`tism::OwnedSharedMemory`] is dropped the shared memory is not necessarily freed. As per
/// the LibC API, shared memory is freed when _all_ users, whether consumer of publisher (which is
/// not a distinction LibC actually makes), have closed their file descriptors for that allocation.
///
/// [`tism`]: crate
/// [`tism::OwnedSharedMemory`]: OwnedSharedMemory
/// [`tism::BorrowedSharedMemory`]: BorrowedSharedMemory
pub struct OwnedSharedMemory<T>(SharedMemory<T>);

impl<T> OwnedSharedMemory<T> {
    /// Locks the [`OwnedSharedMemory`] for writing, returning a [`WriteLockedSharedMemory`] which,
    /// when dropped, will automaticaly unlock the memory.
    ///
    /// [`OwnedSharedMemory`]: OwnedSharedMemory
    /// [`WriteLockedSharedMemory`]: WriteLockedSharedMemory
    pub fn write_lock<'m>(&'m mut self) -> io::Result<WriteLockedSharedMemory<'m, T>> {
        WriteLockedSharedMemory::new(&mut self.0)
    }

    /// Locks the [`OwnedSharedMemory`] for reading, returning a [`ReadLockedSharedMemory`] which,
    /// when dropped, will automatically unlock the memory.
    ///
    /// [`OwnedSharedMemory`]: OwnedSharedMemory
    /// [`ReadLockedSharedMemory`]: ReadLockedSharedMemory
    pub fn read_lock<'m>(&'m mut self) -> io::Result<ReadLockedSharedMemory<'m, T>> {
        ReadLockedSharedMemory::new(&mut self.0)
    }

    /// Lock the shared memory for reading and clone the data inside, returning the result.
    pub fn read(&mut self) -> io::Result<T>
    where
        T: Clone,
    {
        let locked = self.read_lock()?;
        Ok(locked.as_ref().to_owned())
    }

    /// Lock the shared memory for writing and overwrite the data inside.
    pub fn write(&mut self, value: T) -> io::Result<()>
    where
        T: Clone,
    {
        let mut locked = self.write_lock()?;
        *locked.as_mut() = value;
        Ok(())
    }

    /// Get the total number of writes performed on the shared memory. For the purposes of this
    /// function a "write" is one time the read/write lock was locked for writing.
    pub fn total_writes(&self) -> u64 {
        self.0.total_writes()
    }

    /// The size of the portion of shared memory allocated for data.
    pub fn allocated_data_size(&self) -> usize {
        unsafe { (*self.0.allocation).data_size }
    }
}

/// Represents a borrowed (i.e. opened but not created) shared memory allocation. The holder of this
/// `struct` can only read from the shared memory, not write to it.
///
/// Even if the corrosponding [`tism::OwnedSharedMemory`] is dropped, the allocation which the
/// [`tism::BorrowedSharedMemory`] is consuming is not freed until _all_ consumers are also dropped.
/// This means that no syncronization is reauired between the publisher and consumer as far as
/// freeing resources is concerned.
///
/// For a few notes on why this is you can see the docs for [`tism::OwnedSharedMemory`].
///
/// [`tism::OwnedSharedMemory`]: OwnedSharedMemory
/// [`tism::BorrowedSharedMemory`]: BorrowedSharedMemory
pub struct BorrowedSharedMemory<T>(SharedMemory<T>);

impl<T> BorrowedSharedMemory<T> {
    /// Locks the [`BorrowedSharedMemory`] for reading, returning a [`ReadLockedSharedMemory`]
    /// which, when dropped, will automatically unlock the memory.
    ///
    /// [`BorrowedSharedMemory`]: BorrowedSharedMemory
    /// [`ReadLockedSharedMemory`]: ReadLockedSharedMemory
    pub fn read_lock<'m>(&'m mut self) -> io::Result<ReadLockedSharedMemory<'m, T>> {
        ReadLockedSharedMemory::new(&mut self.0)
    }

    /// Lock the shared memory for reading and clone the data inside, returning the result.
    pub fn read(&mut self) -> io::Result<T>
    where
        T: Clone,
    {
        let locked = self.read_lock()?;
        Ok(locked.as_ref().to_owned())
    }

    /// Wraps the [`BorrowedSharedMemory::has_changed`] and [`BorrowedSharedMemory::read`]
    /// functions, returning [`None`] when the allocation has not changed, and returning the value
    /// of [`BorrowedSharedMemory::read`] when it has. This emulates a more "channel-like" behavior.
    ///
    /// ```
    /// let mut shm_owner = tism::create("shm_read_change", 12).unwrap();
    ///
    /// let mut shm = tism::wait_and_open::<i32>("shm_read_change").unwrap();
    ///
    /// if let Some(read_data) = shm.read_change().unwrap() {
    ///     assert_eq!(read_data, 12);
    /// } else {
    ///     // We should have a change to read here.
    ///     panic!();
    /// }
    ///
    /// assert_eq!(None, shm.read_change().unwrap());
    /// shm_owner.write(3).unwrap();
    ///
    /// match shm.read_change().unwrap() {
    ///     Some(i) => {
    ///         assert_eq!(i, 3);
    ///     }
    ///
    ///     None => {
    ///         // We should have a change.
    ///         panic!();
    ///     }
    /// }
    ///
    /// assert_eq!(None, shm.read_change().unwrap());
    /// ```
    pub fn read_change(&mut self) -> io::Result<Option<T>>
    where
        T: Clone,
    {
        if self.has_changed() {
            Ok(Some(self.read()?))
        } else {
            Ok(None)
        }
    }

    /// `true` if the allocation has been written to since the last time this process has read it.
    ///
    /// This function can be used as a cheap and no-lock way of determining if you want to get new
    /// data or if locking the allocation isn't worth doing.
    ///
    /// ```
    /// let mut shm_owner = tism::create("shm_has_changed", 12).unwrap();
    ///
    /// let mut shm = tism::wait_and_open::<i32>("shm_has_changed").unwrap();
    ///
    /// // Initial data which is unread counts as a change!
    /// assert!(shm.has_changed());
    ///
    /// // By reading we clear the change.
    /// let _ = shm.read().unwrap();
    /// assert!(!shm.has_changed());
    ///
    /// // If we perform a write with our owner, then the borrower will see a
    /// // change:
    ///
    /// shm_owner.write(3).unwrap();
    /// assert!(shm.has_changed());
    ///
    /// let _ = shm.read().unwrap();
    /// assert!(!shm.has_changed());
    ///
    /// ```
    pub fn has_changed(&self) -> bool {
        self.0.has_changed()
    }

    /// Gets the [`Duration`] since the last read data had been written to the allocation. This is
    /// _not_ the time since the last read, it is the time since the data _from_ the last read was
    /// _published_, and is partially dependant on the publisher.
    ///
    /// This function returns [`None`] if no read has been performed.
    ///
    /// [`Duration`]: Duration
    /// [`None`]: Option::None
    pub fn staleness(&self) -> Option<Duration> {
        self.0.staleness()
    }

    /// Get the total number of writes performed on the shared memory. For the purposes of this
    /// function a "write" is one time the read/write lock was locked for writing.
    pub fn total_writes(&self) -> u64 {
        self.0.total_writes()
    }

    /// The size of the portion of shared memory allocated for data.
    pub fn allocated_data_size(&self) -> usize {
        unsafe { (*self.0.allocation).data_size }
    }
}

/// Shared memory which has been locked for writing. When this `struct` is dropped it unlocks the
/// memory automatically. [`WriteLockedSharedMemory`] implements [`AsRef`] and [`AsMut`] so that it
/// can be used like any other smart-pointer.
///
/// Write locked memory has exclusive access to the shared memory allocation, and no other process
/// can read from or write to the memory while it is write locked.
///
/// # Example Usage
///
/// ```
/// let mut my_shm = tism::create("shm_write_lock_example", 0).unwrap();
///
/// if let Ok(mut write_locked_shm) = my_shm.write_lock() {
///     let x = *write_locked_shm.as_ref();
///     *write_locked_shm.as_mut() = x + 1;
///     assert_eq!(write_locked_shm.as_ref(), &1);
/// } // Unlock happens here, as `write_locked_shm` drops.
/// ```
///
/// [`AsRef`]: AsRef
/// [`AsMut`]: AsMut
/// [`WriteLockedSharedMemory`]: WriteLockedSharedMemory
pub struct WriteLockedSharedMemory<'m, T>(&'m mut SharedMemory<T>);

impl<'m, T> WriteLockedSharedMemory<'m, T> {
    fn new(shm: &'m mut SharedMemory<T>) -> io::Result<Self> {
        unsafe {
            shm.write_lock()?;
        }

        Ok(WriteLockedSharedMemory(shm))
    }
}

impl<'m, T> AsRef<T> for WriteLockedSharedMemory<'m, T> {
    fn as_ref(&self) -> &T {
        unsafe { &(*self.0.allocation).data }
    }
}

impl<'m, T> AsMut<T> for WriteLockedSharedMemory<'m, T> {
    fn as_mut(&mut self) -> &mut T {
        unsafe { &mut (*self.0.allocation).data }
    }
}

impl<'m, T> Drop for WriteLockedSharedMemory<'m, T> {
    fn drop(&mut self) {
        // it feels wrong but panicing while dropping is a very bad thing to do, and we can't do
        // much if unlocking fails anyways.
        let _ = unsafe { self.0.unlock() };
    }
}

/// Shared memory which has been locked for reading. When this `struct` is dropped it unlocks the
/// memory automatically. [`ReadLockedSharedMemory`] implement [`AsRef`] so that it can be used like
/// a smart pointer.
///
/// Read locked shared memory does not have exclusive access to the shared allocation, but does
/// prevent any process from writing to the memory.
///
/// # Example Usage
///
/// ```
/// // Create shared memory so we can open it later :)
/// let my_shm_owner = tism::create("shm_read_lock_example", 0).unwrap();
///
/// let mut my_shm = tism::open::<i32>("shm_read_lock_example").unwrap();
///
/// if let Ok(read_locked_shm) = my_shm.read_lock() {
///     let x = *read_locked_shm.as_ref();
///     assert_eq!(x, 0);
/// } // Unlock happens here, as `read_locked_shm` drops.
/// ```
///
/// [`AsRef`]: AsRef
/// [`ReadLockedSharedMemory`]: ReadLockedSharedMemory
pub struct ReadLockedSharedMemory<'m, T>(&'m mut SharedMemory<T>);

impl<'m, T> ReadLockedSharedMemory<'m, T> {
    fn new(shm: &'m mut SharedMemory<T>) -> io::Result<Self> {
        unsafe {
            shm.read_lock()?;
        }

        Ok(ReadLockedSharedMemory(shm))
    }
}

impl<'m, T> AsRef<T> for ReadLockedSharedMemory<'m, T> {
    fn as_ref(&self) -> &T {
        unsafe { &(*self.0.allocation).data }
    }
}

impl<'m, T> Drop for ReadLockedSharedMemory<'m, T> {
    fn drop(&mut self) {
        let _ = unsafe { self.0.unlock() };
    }
}

/// Opaque type holding the pointer to both a mutex for syncronization and the data we are concerned
/// with sharing with other processes.
struct SharedMemory<T> {
    /// File descriptor of the shared memory.
    pub(crate) fd: libc::c_int,

    /// The total write count at the event of the last read. Since the write count begins at `1` for
    /// the initial write, this value can be made `0` to consistantly indicate that a read has not
    /// been made.
    pub(crate) last_read_count: u64,

    /// The timestamp of the last read write, i.e. what time was the allocation written to to
    /// produce the data we accessed the last time we read.
    pub(crate) last_read_time: Option<SystemTime>,

    /// The allocation itself, which we should never have our own copy of and will instead only ever
    /// acquire an instance via memory mapping.
    pub(crate) allocation: *mut Allocation<T>,
}

/// Layout of the allocation itself. This is whats shared between processes.
#[repr(C)]
struct Allocation<T> {
    /// Size of the data allocation. For Rust this is the same as the size of the [`Allocation`]
    /// struct.
    ///
    /// [`Allocation`]: Allocation
    pub(crate) data_size: libc::size_t,

    /// Major version of the TISM library used to create the allocation. Mismatched major versions
    /// should result in an error from TISM.
    pub(crate) major_version: u8,

    /// Minor version of the TISM library used to create the allocation. TISM should not produce an
    /// error when opening an allocation of a differing minor version.
    pub(crate) minor_version: u8,

    /// Patch version of the TISM library used to create the allocation. TISM may warn but should
    /// not produce an error for mismatched patches.
    pub(crate) patch_version: u16,

    /// Number of times the [`Allocation`]'s data has been written over. For the purposes of this
    /// field a "write over" corrosponds to one time in which the read/write lock was locked for
    /// writing, so if the publisher process writes multiple times in a single lock, we only count
    /// it once since there would be no way for a consuming process to see that intermediary write.
    ///
    /// The initialization of the allocation with its first value counts as a write.
    ///
    /// [`Allocation`]: Allocation
    pub(crate) total_writes: AtomicU64,

    /// POSIX read/write lock for syncronization. This lock should be used for access to _both_ the
    /// `data` _and_ `timestamp`.
    pub(crate) rw_lock: pthread_rwlock_t,

    /// The time that the read/write lock was last locked for writing.
    pub(crate) timestamp: libc::timespec,

    /// The actual shared data.
    pub(crate) data: T,
}

impl<T> SharedMemory<T> {
    /// Net size of the shared memory allocation such that it may contain our data and a lock.
    const SHARED_MEMORY_SIZE: usize = TISM_OVERHEAD + size_of::<T>();

    /// Create a new allocation of shared memory for a value of `T`. This function is marked unsafe
    /// because it does not initialize the allocation's data or timestamp.
    ///
    /// If a shared memory allocation by the given name already exists, it is deallocated before
    /// creating a new allocation.
    unsafe fn create(name: impl AsRef<Path>) -> io::Result<SharedMemory<T>> {
        let oflags = O_CREAT | O_RDWR | O_TRUNC | O_EXCL;
        let mode = S_IRUSR | S_IWUSR | S_IRGRP | S_IWGRP | S_IROTH;
        let name_bytes = name.as_ref().as_os_str().as_encoded_bytes();
        let mut name_bytes = name_bytes.to_vec();
        name_bytes.push(0);

        #[cfg(target_os = "macos")]
        name_bytes.insert(0, '/' as _);

        unsafe {
            // Open our shared memory as a file descriptor.
            let c_str = name_bytes.as_ptr() as *const libc::c_char;

            // Free the shared memory in case it was already allocated.
            let unlink_err = libc::shm_unlink(c_str);

            if unlink_err != 0
                && io::Error::raw_os_error(&io::Error::last_os_error()) != Some(ENOENT)
            {
                return Err(io::Error::last_os_error());
            }

            #[cfg(target_os = "macos")]
            let fd = shm_open(c_str, oflags, [mode]);
            #[cfg(target_os = "linux")]
            let fd = shm_open(c_str, oflags, mode);

            if fd < 0 {
                return Err(io::Error::last_os_error());
            }

            // Truncate the "file" to the correct size.
            if ftruncate(fd, Self::SHARED_MEMORY_SIZE as i64) < 0 {
                return Err(io::Error::last_os_error());
            }

            // Map the "file" to a new memory address.
            let allocation = libc::mmap(
                ptr::null_mut(),
                Self::SHARED_MEMORY_SIZE,
                libc::PROT_WRITE | libc::PROT_READ,
                libc::MAP_SHARED,
                fd,
                0,
            );

            if allocation == libc::MAP_FAILED {
                return Err(io::Error::last_os_error());
            }

            let allocation = allocation as *mut Allocation<T>;
            (*allocation).data_size = size_of::<T>();
            (*allocation).major_version = MAJOR_VERSION;
            (*allocation).minor_version = MINOR_VERSION;
            (*allocation).patch_version = PATCH_VERSION;
            (*allocation).total_writes = AtomicU64::new(0);

            match pthread_rwlock_init(&raw mut (*allocation).rw_lock, ptr::null()) {
                0 => (),
                e => {
                    return Err(io::Error::from_raw_os_error(e));
                }
            };

            Ok(SharedMemory {
                fd,
                last_read_count: 0,
                last_read_time: None,
                allocation,
            })
        }
    }

    /// Open _but do not create_ a [`SharedMemory`] pointing to `T`.
    ///
    /// [`SharedMemory`]: SharedMemory
    fn open(name: impl AsRef<Path>) -> io::Result<SharedMemory<T>> {
        let name_bytes = name.as_ref().as_os_str().as_encoded_bytes();
        let mut name_bytes = name_bytes.to_vec();
        name_bytes.push(0);

        #[cfg(target_os = "macos")]
        name_bytes.insert(0, '/' as _);

        // For some notes on the inner workings here see `SharedMemory::create`.

        unsafe {
            let c_str = name_bytes.as_ptr() as *const libc::c_char;

            #[cfg(target_os = "macos")]
            let fd = shm_open(c_str, O_RDWR);
            #[cfg(target_os = "linux")]
            let fd = shm_open(c_str, O_RDWR, 0);

            if fd < 0 {
                return Err(io::Error::last_os_error());
            }

            let allocation = libc::mmap(
                ptr::null_mut(),
                size_of::<libc::size_t>(),
                libc::PROT_WRITE | libc::PROT_READ,
                libc::MAP_SHARED,
                fd,
                0,
            );

            if allocation == libc::MAP_FAILED {
                return Err(io::Error::last_os_error());
            }

            let data_size = *(allocation as *const libc::size_t);

            if Self::SHARED_MEMORY_SIZE != TISM_OVERHEAD + data_size {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "Expected ({}) and actual size ({}) of allocation do not match",
                        Self::SHARED_MEMORY_SIZE,
                        TISM_OVERHEAD + data_size
                    ),
                ));
            }

            munmap(allocation, size_of::<libc::size_t>());

            let allocation = libc::mmap(
                ptr::null_mut(),
                TISM_OVERHEAD + data_size,
                libc::PROT_WRITE | libc::PROT_READ,
                libc::MAP_SHARED,
                fd,
                0,
            );

            if allocation == libc::MAP_FAILED {
                return Err(io::Error::last_os_error());
            }

            let allocation = allocation as *mut Allocation<T>;

            if (*allocation).major_version != MAJOR_VERSION {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "TISM major version mismatch",
                ));
            }

            Ok(SharedMemory {
                fd,
                last_read_count: 0,
                last_read_time: None,
                allocation,
            })
        }
    }

    /// `true` if the allocation has been written to since the last time this process has read it.
    /// This does not guarantee that the actual data has changed, only that the allocation has been
    /// written to.
    fn has_changed(&self) -> bool {
        self.total_writes() > self.last_read_count
    }

    /// Gets the [`Duration`] since the last read data had been written to the allocation. This is
    /// _not_ the time since the last read, it is the time since the data _from_ the last read was
    /// published, and is partially dependant on the publisher.
    ///
    /// This function returns [`None`] if no read has been performed.
    ///
    /// [`Duration`]: Duration
    /// [`None`]: Option::None
    fn staleness(&self) -> Option<Duration> {
        self.last_read_time.and_then(|t| t.elapsed().ok())
    }

    /// Get the total writes field of the shared memory's [`Allocation`].
    ///
    /// [`Allocation`]: Allocation
    fn total_writes(&self) -> u64 {
        unsafe {
            (*self.allocation)
                .total_writes
                .load(atomic::Ordering::SeqCst)
        }
    }

    unsafe fn write_lock(&mut self) -> io::Result<()> {
        unsafe {
            if pthread_rwlock_wrlock(&raw mut (*self.allocation).rw_lock) != 0 {
                return Err(io::Error::last_os_error());
            }

            if libc::clock_gettime(CLOCK_MONOTONIC, &mut (*self.allocation).timestamp) != 0 {
                return Err(io::Error::last_os_error());
            }

            let _ = (*self.allocation)
                .total_writes
                .fetch_add(1, atomic::Ordering::SeqCst);

            Ok(())
        }
    }

    unsafe fn read_lock(&mut self) -> io::Result<()> {
        unsafe {
            if pthread_rwlock_rdlock(&raw mut (*self.allocation).rw_lock) != 0 {
                return Err(io::Error::last_os_error());
            }

            let time = (*self.allocation).timestamp;

            self.last_read_time = Some(
                SystemTime::UNIX_EPOCH
                    + Duration::from_secs(time.tv_sec as _)
                    + Duration::from_nanos(time.tv_nsec as _),
            );
        }

        self.last_read_count = self.total_writes();

        Ok(())
    }

    unsafe fn unlock(&mut self) -> io::Result<()> {
        unsafe {
            match pthread_rwlock_unlock(&raw mut (*self.allocation).rw_lock) {
                0 => Ok(()),
                e => Err(io::Error::from_raw_os_error(e)),
            }
        }
    }
}

impl<T> Drop for SharedMemory<T> {
    fn drop(&mut self) {
        unsafe {
            close(self.fd);
            munmap(
                &raw mut (*self.allocation).rw_lock as _,
                Self::SHARED_MEMORY_SIZE,
            );
        }
    }
}
