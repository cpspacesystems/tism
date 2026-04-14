"""
Python wrapper for TISM-C. This wrapper hides the unsafe API which is available
in C due to the way Python would be forced to interact with C pointers.

TISM works by allocating shared memory using a name, publishers can write to an
allocation and must create one using the `tism.create()` functon. Consumers can
then open the allocation using `tism.open()`, at which point they can read
whatever data the publisher has written there.

All TISM allocation use read/write locks, so many consumers may read an
allocation at once, but only one writer may write at a time (only one publisher
should ever exist anyways). Consumers may not read when a publisher is writing
and vice versa.

For Python users of TISM you must serialize you data to/from `bytes`, as Python
has, in general, a very loose notion of the size of a type. For Rust and C the
bytes will likely just be the raw bytes of the type the allocation is meant to
store, and Python users should work with their `bytes` as such.
"""

import time
from dataclasses import dataclass
from datetime import datetime
from typing import Optional
from _tism import ffi, lib


@dataclass
class _TismOwnedSharedMemory:
    """
    A TISM shared memory allocation which this process has created. Holding an
    instance of this class means that you are the publisher, and the only party
    which may write to or read from the memory allocation.
    """

    _shm: ffi.CData

    def write(self, value: bytes):
        """
        Write the given bytes into the shared memory allocation by cloning them.
        This function write locks the allocation, and will block until it can do
        so, then unlocks before returning.
        """

        if self._shm.allocation.data_size != len(value):
            raise RuntimeError("Given value is not the same size as allocation")

        value_ptr = ffi.new("char[]", bytes(value))
        _raise_tism_error(lib.tism_owned_write(self._shm, value_ptr))

    def read(self) -> bytes:
        """
        Read from the memory allocation. This function will read lock the
        allocation, and block until it can do so, then unlock before returning.
        """

        value_ptr = ffi.new("char[]", self._shm.allocation.data_size)
        _raise_tism_error(lib.tism_owned_read(self._shm, value_ptr))
        buf = ffi.buffer(value_ptr, self._shm.allocation.data_size)
        return bytes(buf)

    def get_total_writes(self) -> int:
        """
        Get the total number of writes made to this allocation. For the purposes
        of this function, one write is one time that the allocation was locked
        for writing, even if in the time it was locked it was overwritten
        multiple times, since a consumer will only see the new data after it is
        unlocked.
        """

        return lib.tism_owned_get_total_writes(self._shm)

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_value, traceback):
        self.__del__()

    def __del__(self):
        lib.tism_owned_close(self._shm)
        self._shm = ffi.cast("void*", 0)


@dataclass
class _TismBorrowedSharedMemory:
    """
    A TISM shared memory allocation which this process has opened, not created.
    Holding an instance of this class means that you are a consumer, and can
    only read from the allocation.
    """

    _shm: ffi.CData

    def read(self) -> bytes:
        """
        Read from the memory allocation. This function will read lock the
        allocation, and block until it can do so, then unlock before returning.
        """

        value_ptr = ffi.new("char[]", self._shm.allocation.data_size)
        _raise_tism_error(lib.tism_owned_read(self._shm, value_ptr))
        buf = ffi.buffer(value_ptr, self._shm.allocation.data_size)
        return bytes(buf)

    def read_change(self) -> bytes | None:
        """
        Optionally read the data in the allocation, depending on whether or not
        it has changed. If the allocation has been written to since it was last
        read by this process, this function will return the data from within the
        allocation.
        """

        if self.has_changed():
            return self.read()

        return None

    def has_changed(self) -> bool:
        """
        Returns `True` if the allocation has been written to since the last time
        this process has read the allocation. This does not guarantee that the
        actual data has changed.
        """

        return lib.tism_borrowed_has_changed(self._shm)

    def staleness_nanos(self) -> int:
        """
        Gets the staleness of the last read write, as in, the duration since the
        last write that this process has read, in a whole number of nanoseconds.
        """

        return lib.tism_borrowed_staleness(self._shm)

    def staleness(self) -> float:
        """
        Gets the staleness of the last read write, as in, the duration since the
        last write that this process has read, in fractional seconds. This
        function cannot exceed nanosecond precision.
        """

        return float(self.staleness_nanos()) / 1_000_000_000.0

    def get_total_writes(self) -> int:
        """
        Get the total number of writes made to this allocation. For the purposes
        of this function, one write is one time that the allocation was locked
        for writing, even if in the time it was locked it was overwritten
        multiple times, since a consumer will only see the new data after it is
        unlocked.
        """

        return lib.tism_borrowed_get_total_writes(self._shm)

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_value, traceback):
        self.__del__()

    def __del__(self):
        lib.tism_borrowed_close(self._shm)
        self._shm = ffi.cast("void*", 0)


def create(name: str, init: bytes) -> _TismOwnedSharedMemory:
    """
    Create a new TISM shared memory allocation with the given name and given
    initial data. Do not include a leading "/" in the name.
    """

    c_str = _create_c_str(name)
    init_ptr = ffi.new("char[]", bytes(init))  # use char[] as void* standin

    shm = ffi.new("struct _tism_shared_memory*")
    _raise_tism_error(lib.tism_create(shm, c_str, init_ptr, len(init)))

    return _TismOwnedSharedMemory(shm)


def open(name: str) -> _TismBorrowedSharedMemory:
    """
    Open an existing shared memory allocation by the given name
    """

    c_str = _create_c_str(name)

    shm = ffi.new("struct _tism_shared_memory*")
    _raise_tism_error(lib.tism_open(shm, c_str))

    return _TismBorrowedSharedMemory(shm)


def wait_and_open(name: str) -> _TismBorrowedSharedMemory:
    """
    Open an existing shared memory allocation by the given name
    """

    c_str = _create_c_str(name)

    shm = ffi.new("struct _tism_shared_memory*")
    _raise_tism_error(lib.tism_wait_and_open(shm, c_str))

    return _TismBorrowedSharedMemory(shm)


def _raise_tism_error(err: ffi.CData):
    """
    Check our C-style result enum errors into Python exception handling. Raises
    an exception if the given result is not `TISM_OK`.
    """

    if err == lib.TISM_OK:
        return

    raise RuntimeError(f"TISM gave error: {err}")


def _create_c_str(s: str):
    """
    Create a C-string from the given python `str`
    """

    return ffi.new("char[]", (s + '\0').encode())
