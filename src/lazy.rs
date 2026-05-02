//! Wrappers for [`tism`]'s existing shared memory interfaces with the added ability for the
//! creation of the holding type and allocation of the actual memory to be done at seperate times,
//! allowing for the holding type to be created first, then when it gets its first value for the
//! allocation to be made.
//!
//! This allows for a lazy pattern of shared memory usage, which may be ideal for publishing data
//! gathered via IO in discrete frames. The use of this API to interact with an allocation with one'
//! process does not affect another process' ability to use another API.
//!
//! If laziness is not required then all this interface for shared memory will do is introduce
//! checks on reads which complicates error handling and make zero-copy interaction harder
//!
//! # Example
//!
//! ```
//! #[derive(Debug, PartialEq, Clone, Copy)]
//! struct MyData {
//!     field_1: u32,
//!     field_2: f32,
//! }
//!
//! // This won't allocate any memory.
//! let mut lazy_shm = tism::lazy::create("my_lazy_shm");
//!
//! // The memory is allocated here.
//! lazy_shm.write(MyData { field_1: 3, field_2: 2.18 }).unwrap();
//!
//! // Memory is overwritten here.
//! lazy_shm.write(MyData { field_1: 5, field_2: 0f32 }).unwrap();
//!
//! // Because we've written to our shared memory we know its allocated, so this
//! // should always succeed and return `Some`.
//! let mut strict_shm: tism::OwnedSharedMemory<MyData> = lazy_shm.strict().unwrap();
//! let read_value = strict_shm.read().unwrap();
//!
//! assert_eq!(read_value, MyData { field_1: 5, field_2: 0f32 });
//! ```
//!
//! [`tism`]: crate

use crate::OwnedSharedMemory;
use std::io;
use std::path::Path;

/// Create a new, lazy, owned shared memory instance. Notice that this function does not return a
/// [`io::Result`], and does not perform IO. This function will not _allocate_ memory, it only sets
/// up the [`LazyOwnedSharedMemory`] so that it could do so later.
///
/// [`io::Result`]: io::Result
/// [`LazyOwnedSharedMemory`]: LazyOwnedSharedMemory
pub fn create<T, P>(name: P) -> LazyOwnedSharedMemory<T, P>
where
    P: AsRef<Path>,
{
    LazyOwnedSharedMemory::Unallocated(name)
}

/// A "lazy" version of [`OwnedSharedMemory`], which will not actually allocate memory until the
/// first write. As a result this interface does not need an initial value, which is its key
/// advantage, however it only supports a subset of the interface [`OwnedSharedMemory`] does. To
/// overcome this limitation the lazy memory can be forced allocated via
/// [`LazyOwnedSharedMemory::allocate`] and then an [`OwnedSharedMemory`] may be extracted by
/// [`LazyOwnedSharedMemory::strict`].
///
/// # Example
///
/// ```
/// #[derive(Debug, PartialEq, Clone, Copy)]
/// struct MyData {
///     field_1: u32,
///     field_2: f32,
/// }
///
/// // This won't allocate any memory.
/// let mut lazy_shm = tism::lazy::create("my_lazy_owned_shm");
/// assert!(!lazy_shm.has_allocated());
///
/// // The memory is allocated here.
/// lazy_shm.write(MyData { field_1: 3, field_2: 2.18 }).unwrap();
/// assert!(lazy_shm.has_allocated());
///
/// // Memory is overwritten here.
/// lazy_shm.write(MyData { field_1: 5, field_2: 0f32 }).unwrap();
///
/// // Because we've written to our shared memory we know its allocated, so this
/// // should always succeed and return `Some`.
/// let mut strict_shm: tism::OwnedSharedMemory<MyData> = lazy_shm.strict().unwrap();
/// let read_value = strict_shm.read().unwrap();
///
/// assert_eq!(read_value, MyData { field_1: 5, field_2: 0f32 });
/// ```
///
/// [`OwnedSharedMemory`]: OwnedSharedMemory
/// [`LazyOwnedSharedMemory`]: LazyOwnedSharedMemory
/// [`LazyOwnedSharedMemory::allocate`]: LazyOwnedSharedMemory::allocate
/// [`LazyOwnedSharedMemory::strict`]: LazyOwnedSharedMemory::strict
pub enum LazyOwnedSharedMemory<T, P>
where
    P: AsRef<Path>,
{
    Allocated(OwnedSharedMemory<T>),
    Unallocated(P),
}

impl<T, P> LazyOwnedSharedMemory<T, P>
where
    P: AsRef<Path>,
{
    /// Attempt to make the [`LazyOwnedSharedMemory`] into its "strict" counterpart. This function
    /// returns an [`OwnedSharedMemory`] if the [`LazyOwnedSharedMemory`] has already allocated,
    /// otherwise [`None`] is returned.
    ///
    /// Avoid using this function before either checking if the memory has been allocated, or
    /// writing to it via [`LazyOwnedSharedMemory::write`] or [`LazyOwnedSharedMemory::allocate`].
    ///
    /// [`None`]: Option::None
    /// [`OwnedSharedMemory`]: OwnedSharedMemory
    /// [`LazyOwnedSharedMemory`]: LazyOwnedSharedMemory
    /// [`LazyOwnedSharedMemory::write`]: LazyOwnedSharedMemory::write
    /// [`LazyOwnedSharedMemory::allocate`]: LazyOwnedSharedMemory::allocate
    pub fn strict(self) -> Option<OwnedSharedMemory<T>> {
        match self {
            Self::Allocated(shm) => Some(shm),
            _ => None,
        }
    }

    /// Check whether the [`LazyOwnedSharedMemory`] has allocated its memory or not.
    ///
    /// [`LazyOwnedSharedMemory`]: LazyOwnedSharedMemory
    pub fn has_allocated(&self) -> bool {
        match self {
            Self::Allocated(_) => true,
            Self::Unallocated(_) => false,
        }
    }

    /// Force the given [`LazyOwnedSharedMemory`] to allocate its memory, initializing it with the
    /// given value of `T`. If the [`LazyOwnedSharedMemory`] has already allocated its memory then
    /// this function is a no-op and simply returns [`Ok`].
    ///
    /// To create _or_ overwrite a [`LazyOwnedSharedMemory`] use [`LazyOwnedSharedMemory::write`].
    ///
    /// [`Ok`]: Ok
    /// [`LazyOwnedSharedMemory`]: LazyOwnedSharedMemory
    /// [`LazyOwnedSharedMemory::write`]: LazyOwnedSharedMemory::write
    pub fn allocate(&mut self, init: T) -> io::Result<()> {
        match self {
            Self::Allocated(_) => Ok(()),
            Self::Unallocated(name) => {
                *self = LazyOwnedSharedMemory::Allocated(crate::create(name, init)?);
                Ok(())
            }
        }
    }
}

impl<T, P> LazyOwnedSharedMemory<T, P>
where
    T: Clone,
    P: AsRef<Path>,
{
    /// Write to the [`LazyOwnedSharedMemory`], allocating the memory if it hasn't already been, and
    /// overwriting the existing memory if it has.
    ///
    /// [`LazyOwnedSharedMemory`]: LazyOwnedSharedMemory
    pub fn write(&mut self, value: T) -> io::Result<()> {
        match self {
            Self::Allocated(shm) => shm.write(value),
            Self::Unallocated(_) => self.allocate(value),
        }
    }
}
