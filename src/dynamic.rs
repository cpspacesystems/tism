//! This module is for treating [`tism`] allocation _as if_ they were dynamically size objects. This
//! module only provides consumer APIs and is not for general use. The main use case for something
//! like this is dealing with a number of [`tism`] allocations and not caring to extract any
//! particular data from them but rather to record the allocation as a whole or to forward it to
//! another means of communication.
//!
//! Avoid using this module if possible.
//!
//! [`tism`]: crate

use crate::{Allocation, MAJOR_VERSION};
use libc::{O_RDWR, close, munmap, pthread_rwlock_rdlock, pthread_rwlock_unlock, shm_open};
use std::{io, path::Path, ptr, slice};

/// Open a shared memory allocation for reading with an unknown size. When reading from this
/// allocation [`tism`] will look up the size of the data in the header of the allocation.
///
/// [`tism`]: crate
pub fn open(name: impl AsRef<Path>) -> io::Result<DynamicBorrowedSharedMemory> {
    let shm = SharedMemory::open(name)?;
    Ok(DynamicBorrowedSharedMemory(shm))
}

/// A shared memory allocation which is open for reading (like with [`tism::BorrowedSharedMemory`])
/// but without a size known ahead of time. This type can be used to interact with allocations of an
/// unkown size, but as a cost for this we work in a less type-ful setting, and must take raw bytes
/// out of the allocation instead of a generic type.
///
/// [`tism::BorrowedSharedMemory`]: crate::BorrowedSharedMemory
pub struct DynamicBorrowedSharedMemory(SharedMemory);

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

    /// The size of the portion of shared memory allocated for data.
    pub fn allocated_data_size(&self) -> usize {
        unsafe { (*self.0.allocation).data_size }
    }
}

struct SharedMemory {
    /// File descriptor of the shared memory.
    pub(crate) fd: libc::c_int,

    /// The allocation itself.
    pub(crate) allocation: *mut Allocation<u8>,
}

impl SharedMemory {
    fn open(name: impl AsRef<Path>) -> io::Result<SharedMemory> {
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

            munmap(allocation, size_of::<libc::size_t>());

            let allocation = libc::mmap(
                ptr::null_mut(),
                crate::TISM_OVERHEAD + data_size,
                libc::PROT_WRITE | libc::PROT_READ,
                libc::MAP_SHARED,
                fd,
                0,
            );

            if allocation == libc::MAP_FAILED {
                return Err(io::Error::last_os_error());
            }

            let allocation = allocation as *mut Allocation<u8>;

            if (*allocation).major_version != MAJOR_VERSION {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "TISM major version mismatch",
                ));
            }

            Ok(SharedMemory { fd, allocation })
        }
    }

    unsafe fn read_lock(&mut self) -> io::Result<()> {
        unsafe {
            match pthread_rwlock_rdlock(&raw mut (*self.allocation).rw_lock) {
                0 => Ok(()),
                e => Err(io::Error::from_raw_os_error(e)),
            }
        }
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

impl Drop for SharedMemory {
    fn drop(&mut self) {
        unsafe {
            close(self.fd);
            munmap(
                &raw mut (*self.allocation).rw_lock as _,
                crate::TISM_OVERHEAD + (*self.allocation).data_size,
            );
        }
    }
}
