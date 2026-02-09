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
//! let mut my_shm = tism::create("shm_owned_example", 0).unwrap();
//!
//! if let Ok(mut lock) = my_shm.write_lock() {
//!     // by pattern matching on the lock result, we confine our lock to the
//!     // scope of this if statement
//!     let x = *lock.as_ref();
//!     *lock.as_mut() = x + 1;
//!
//!     assert_eq!(lock.as_ref(), &1);
//! }
//! ```
//!
//! This way is simpler, but takes ownership of the given value of `T`. If you were to make many
//! changes in sequence it would rapidly lock and unlock the shared memory. If you make only one
//! write then this method is perfect, for more complicated application use the method outline in
//! the previous example.
//!
//! ```
//! let mut my_shm = tism::create("shm_write_onestep_example", 0).unwrap();
//!
//! // locking happens internally
//! my_shm.write(1).unwrap();
//!
//! let x = my_shm.read().unwrap();
//! assert_eq!(x, 1);
//! ```
//!
//! ## Consumer
//!
//! This is the recommended way for consumers to use shared memory, it allows direct, _zero-copy_
//! access to the shared memory, allowing you to more efficiently extract particular fields.
//!
//! ```
//! // Create shared memory so we can open it later :)
//! let my_shm_owner = tism::create("shm_zerocopy_consumer_example", 0).unwrap();
//!
//! let mut my_shm = tism::open::<i32>("shm_zerocopy_consumer_example").unwrap();
//!
//! if let Ok(lock) = my_shm.read_lock() {
//!     let x = *lock.as_ref();
//!     assert_eq!(lock.as_ref(), &0);
//! }
//! ```
//!
//! If `T` is [`Clone`] you can also use this simpler method, though it is less efficient:
//!
//! ```
//! let my_shm_owner = tism::create("shm_clone_consumer_example", 0).unwrap();
//!
//! let mut my_shm = tism::open::<i32>("shm_clone_consumer_example").unwrap();
//!
//! let x = my_shm.read().unwrap();
//! assert_eq!(x, 0);
//! ```
//!
//! [`tism`]: crate
//! [`tism::create`]: create
//! [`tism::open`]: open
//! [`OwnedSharedMemory`]: OwnedSharedMemory
//! [`BorrowedSharedMemory`]: BorrowedSharedMemory
//! [`Sized`]: Sized

use libc::{
    self, ENOENT, O_CREAT, O_EXCL, O_RDWR, O_TRUNC, S_IRGRP, S_IROTH, S_IRUSR, S_IWGRP, S_IWUSR,
    close, ftruncate, munmap, pthread_rwlock_init, pthread_rwlock_rdlock, pthread_rwlock_t,
    pthread_rwlock_unlock, pthread_rwlock_wrlock, shm_open,
};
use std::{io, path::Path, ptr};

#[cfg(test)]
mod tests;

/// Create a new shared memory allocaton, which you will own, initialized to the given value of `T`.
/// If a shared memory allocation by the given name already exists this is not considered an error,
/// since its possible that we are reaquiring an allocation which we failed to clean up earlier, so
/// care should be taken not to have two processes call [`tism::create`] with the same name so long
/// as their usage overlaps.
///
/// See docs on [`tism::OwnedSharedMemory`] for info on how resources are freed.
///
/// [`tism::create`]: create
/// [`tism::OwnedSharedMemory`]: OwnedSharedMemory
pub fn create<T>(name: impl AsRef<Path>, init: T) -> io::Result<OwnedSharedMemory<T>> {
    let mut shm = unsafe { SharedMemory::create(name) }?;

    unsafe {
        shm.write_lock()?;
        *shm.data = init;
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
        unsafe { &*self.0.data }
    }
}

impl<'m, T> AsMut<T> for WriteLockedSharedMemory<'m, T> {
    fn as_mut(&mut self) -> &mut T {
        unsafe { &mut *self.0.data }
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
        unsafe { &*self.0.data }
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

    /// POSIX read/write lock for syncronization.
    pub(crate) rw_lock: *mut pthread_rwlock_t,

    /// The actual shared data, this pointer lives in the same allocation as `mutex`, and should be
    /// a constant offset from it.
    pub(crate) data: *mut T,
}

impl<T> SharedMemory<T> {
    /// Net size of the shared memory allocation such that it may contain our data and a mutex.
    const SHARED_MEMORY_SIZE: usize = size_of::<pthread_rwlock_t>() + size_of::<T>();

    /// Create a new allocation of shared memory for a value of `T`. This function is marked unsafe
    /// because it does not initialize the allocation.
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
            #[cfg(target_os = "macos")]
            let c_str = name_bytes.as_ptr() as *const libc::c_char;
            #[cfg(target_os = "linux")]
            let c_str = name_bytes.as_ptr() as *const u8;

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

            // Divide the memory into a mutex and actual data, then initialize the mutex and data.
            let rw_lock = allocation as *mut pthread_rwlock_t;
            let data = allocation.byte_offset(size_of::<pthread_rwlock_t>() as isize) as *mut T;

            match pthread_rwlock_init(rw_lock, ptr::null()) {
                0 => (),
                e => {
                    return Err(io::Error::from_raw_os_error(e));
                }
            };

            Ok(SharedMemory { fd, rw_lock, data })
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
            #[cfg(target_os = "macos")]
            let c_str = name_bytes.as_ptr() as *const libc::c_char;
            #[cfg(target_os = "linux")]
            let c_str = name_bytes.as_ptr() as *const u8;

            #[cfg(target_os = "macos")]
            let fd = shm_open(c_str, O_RDWR);
            #[cfg(target_os = "linux")]
            let fd = shm_open(c_str, O_RDWR, 0);

            if fd < 0 {
                return Err(io::Error::last_os_error());
            }

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

            let rw_lock = allocation as *mut pthread_rwlock_t;
            let data = allocation.byte_offset(size_of::<pthread_rwlock_t>() as isize) as *mut T;

            Ok(SharedMemory { fd, rw_lock, data })
        }
    }

    unsafe fn write_lock(&mut self) -> io::Result<()> {
        unsafe {
            match pthread_rwlock_wrlock(self.rw_lock) {
                0 => Ok(()),
                e => Err(io::Error::from_raw_os_error(e)),
            }
        }
    }

    unsafe fn read_lock(&mut self) -> io::Result<()> {
        unsafe {
            match pthread_rwlock_rdlock(self.rw_lock) {
                0 => Ok(()),
                e => Err(io::Error::from_raw_os_error(e)),
            }
        }
    }

    unsafe fn unlock(&mut self) -> io::Result<()> {
        unsafe {
            match pthread_rwlock_unlock(self.rw_lock) {
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
            munmap(self.rw_lock as _, Self::SHARED_MEMORY_SIZE);
        }
    }
}
