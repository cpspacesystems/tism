//! This module is for treating [`tism`] allocation _as if_ they were dynamically size objects. This
//! module only provides consumer APIs and is not for general use. The main use case for something
//! like this is dealing with a number of [`tism`] allocations and not caring to extract any
//! particular data from them but rather to record the allocation as a whole or to forward it to
//! another means of communication.
//!
//! Avoid using this module if possible.
//!
//! [`tism`]: crate

use crate::{OpenMode, SharedMemory};
use std::{io, path::Path, slice, time::Duration};

/// Create a lazy shared memory allocation with a dynamic size.
pub fn create(name: impl AsRef<Path>, size: usize) -> io::Result<OwnedDynamicSharedMemory> {
    let shm = unsafe { SharedMemory::create_with_size(name, size)? };
    Ok(OwnedDynamicSharedMemory(shm))
}

/// Open a shared memory allocation for reading with an unknown size. When reading from this
/// allocation [`tism`] will look up the size of the data in the header of the allocation.
///
/// [`tism`]: crate
pub fn open(name: impl AsRef<Path>) -> io::Result<DynamicBorrowedSharedMemory> {
    let shm = SharedMemory::open(name, OpenMode::Dynamic)?;
    Ok(DynamicBorrowedSharedMemory(shm))
}

/// Open a shared memory allocation for reading with an unknown size, trying repeatedly until the
/// allocation becomes available. When reading from this allocation [`tism`] will look up the size
/// of the data in the header of the allocation.
///
/// [`tism`]: crate
pub fn wait_and_open(name: impl AsRef<Path>) -> io::Result<DynamicBorrowedSharedMemory> {
    loop {
        match SharedMemory::open(name.as_ref(), OpenMode::Dynamic) {
            Ok(shm) => return Ok(DynamicBorrowedSharedMemory(shm)),

            Err(io_err) => match io_err.kind() {
                io::ErrorKind::NotFound => continue,
                _ => return Err(io_err),
            },
        }
    }
}

/// Owned shared memory, which has a data size determined at runtime rather than by a fixed type
/// variable. This is a counterpart to [`tism::OwnedSharedMemory`], but rather than deriving the
/// size of the allocation from a type parameter, it is explicitly given when created.
///
/// [`tism::OwnedSharedMemory`]: tism::OwnedSharedMemory
pub struct OwnedDynamicSharedMemory(SharedMemory<u8>);

/// A shared memory allocation which is open for reading (like with [`tism::BorrowedSharedMemory`])
/// but without a size known ahead of time. This type can be used to interact with allocations of an
/// unkown size, but as a cost for this we work in a less type-ful setting, and must take raw bytes
/// out of the allocation instead of a generic type.
///
/// [`tism::BorrowedSharedMemory`]: crate::BorrowedSharedMemory
pub struct DynamicBorrowedSharedMemory(SharedMemory<u8>);

impl OwnedDynamicSharedMemory {
    /// Write the given raw data (as a [`Vec`] of `u8`) to the [`OwnedDynamicSharedMemory`]. The
    /// length of the given [`Vec`] must match the data size of the allocation, if it does not an
    /// [`Err`] is returned.
    ///
    /// [`Vec`]: Vec
    /// [`Err`]: Err
    /// [`OwnedDynamicSharedMemory`]: OwnedDynamicSharedMemory
    pub fn write(&mut self, data: Vec<u8>) -> io::Result<()> {
        let shm = &mut self.0;

        unsafe {
            let alloc = &(*shm.allocation);

            if data.len() == alloc.data_size {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "Given `Vec` and `OwnedDynamicSharedMemory` have incompatable sizes ({} vs {})",
                        data.len(),
                        alloc.data_size
                    ),
                ));
            }

            shm.write_lock()?;

            let slice = std::slice::from_raw_parts_mut(
                &raw mut (*shm.allocation).data,
                (*shm.allocation).data_size,
            );

            slice.copy_from_slice(data.as_slice());

            shm.unlock()?;
        };

        Ok(())
    }
}

impl DynamicBorrowedSharedMemory {
    /// Read the shared memory, getting a [`Vec`] of `u8` bytes
    ///
    /// [`Vec`]: Vec
    pub fn read(&mut self) -> io::Result<Vec<u8>> {
        let shm = &mut self.0;

        unsafe {
            shm.read_lock()?;

            let slice =
                slice::from_raw_parts(&raw mut (*shm.allocation).data, (*shm.allocation).data_size);
            let vec = slice.to_vec();

            shm.unlock()?;

            Ok(vec)
        }
    }

    /// Wraps the [`BorrowedSharedMemory::has_changed`] and [`BorrowedSharedMemory::read`]
    /// functions, returning [`None`] when the allocation has not changed, and returning the value
    /// of [`BorrowedSharedMemory::read`] when it has. This emulates a more "channel-like" behavior.
    ///
    /// ```
    /// let mut shm_owner = tism::create::<[u8; 1]>("dyn_read_change", [12u8]).unwrap();
    ///
    /// let mut shm = tism::dynamic::open("dyn_read_change").unwrap();
    ///
    /// if let Some(read_data) = shm.read_change().unwrap() {
    ///     assert_eq!(read_data, [12u8]);
    /// } else {
    ///     // We should have a change to read here.
    ///     panic!();
    /// }
    ///
    /// assert_eq!(None, shm.read_change().unwrap());
    /// shm_owner.write([3u8]).unwrap();
    ///
    /// match shm.read_change().unwrap() {
    ///     Some(i) => {
    ///         assert_eq!(i, [3u8]);
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
    pub fn read_change(&mut self) -> io::Result<Option<Vec<u8>>> {
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
    /// let mut shm_owner = tism::create::<[u8; 1]>("dyn_has_changed", [12u8]).unwrap();
    ///
    /// let mut shm = tism::dynamic::open("dyn_has_changed").unwrap();
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
    /// shm_owner.write([3u8]).unwrap();
    /// assert!(shm.has_changed());
    ///
    /// let _ = shm.read().unwrap();
    /// assert!(!shm.has_changed());
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
