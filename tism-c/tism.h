/*
 * C API for TISM. Most of the functions here are meant to be analogs to their Rust counterparts and
 * are fully interoperable. Unlike the Rust API however, the C API cannot use generics, as a result
 * any functions which would deal with generics in Rust will deal with `void` pointers and lengths/
 * sizes.
 *
 * All functions and types in this header have prefixes, and they have a few different meanings:
 *  - `tism_` - normal function.
 *  - `_tism_` - "private", try not to use these directly. Functions with this prefix are implicitly
 *    unsafe and may require setup and/or cleanup.
 *  - `tism_unsafe_` - unsafe function in some sense, read the documentation for these very
 *    carefully if you opt to use them. These functions can speed up your code, but at the cost of
 *    TISM granting you less assurances.
 */

#ifndef _TISM_H
#define _TISM_H

#include <stddef.h>
#include <pthread.h>
#include <sys/mman.h>

#define TISM_MAJOR_VERSION 0
#define TISM_MINOR_VERSION 0
#define TISM_PATCH_VERSION 0

/*
 * Monadic-bind like operation for `tism_result_t`. Returns early if the result is an error, and
 * continues execution as normal if it is `TISM_OK`.
 */
#define TISM_MBIND(expr) { tism_result_t err = expr; if (err != TISM_OK) { return err; } }

#if defined(__APPLE__)
#include <sys/posix_shm.h>
/* minus one for null terminator, minus one for leading slash */
#define TISM_MAX_NAME_LENGTH (PSHMNAMLEN - 2)
#elif defined(__linux__)
/* minux one for null terminator, though length here is arbitrary */
#define TISM_MAX_NAME_LENGTH 255
#endif

/*
 * A TISM shared memory allocation which this process has created. Holding this type means that you
 * are the publishing process, and the only process with write capabilities.
 */
typedef struct _tism_shared_memory tism_owned_shared_memory_t;

/*
 * A TISM shared memory allocation which this process has opened, but was created by another
 * process. Holding this type means you are a consumer and may only read the memory.
 *
 * Never cast this to either a `tism_owned_shared_memory_t` or a `struct _tism_shared_memory`, doing
 * so could be claiming that this process has write permissions to the memory and could result in
 * undesireable behavior from TISM. The types are here to help, use them correctly.
 */
typedef struct _tism_shared_memory tism_borrowed_shared_memory_t;

/*
 * Common struct for TISM shared memory. Avoid direct use.
 */
struct _tism_shared_memory {
	int fd;
	struct _tism_allocation* allocation;
};

/*
 * The layout of a TISM shared memory allocation.
 */
struct _tism_allocation {
	size_t data_size;
	uint8_t major_version;
	uint8_t minor_version;
	uint16_t patch_version;
	pthread_rwlock_t rw_lock;
	char data[]; /* This field just marks the first byte of data. */
};

/*
 * Errors that may occur while using TISM, all functions return this type.
 */
typedef enum {
	TISM_OK = 0,  /* No error, all good :) */

	TISM_INVALID_ARGUMENT, /* Invalid argument passed to function. */
	TISM_BAD_PERMISSIONS,  /* The OS got mad at us :( */
	TISM_INTERUPTED,       /* Got a signal which interupted an operation. */
	TISM_UNSUPPORTED,      /* A needed operation was not supported. */
	TISM_TOO_MANY_FDS,     /* This process already has open the max number of file descriptors. */
	TISM_FILE_TABLE,       /* The system file table is at capacity. */
	TISM_NO_SPACE,         /* Insufficiant space to allocate recourse. */
	TISM_TOO_BIG, 		   /* Required allocation exceeds system maximum. */
	TISM_VERSION_MISMATCH, /* Attempted to open an allocaton with a mismatched major version. */

	TISM_UNKNOWN,  /* An unknown error occured. */
} tism_result_t;

/*
 * Create (and take ownership of) a new shared memory allocation. The allocation will be initialized
 * with the given data, comprised of a `void` pointer and `size_t` length of the data in bytes.
 *
 * This function is the analog for the `tism::create` function in Rust, and aims to recreate its
 * behavior as closely as possible.
 *
 * TISM will save the size you give here, and functions which use the `tism_owned_shared_memory_t`
 * will not take it as a parameter.
 */
tism_result_t tism_create(volatile tism_owned_shared_memory_t* shm, char* name, const void* data, size_t n);

/*
 * Open a shared memory allocation for reading. This function recovers the size of the allocation
 * from its file descriptor, and this value is saved an used for all other functions which use a
 * `tism_borrowed_shared_memory_t`.
 */
tism_result_t tism_open(volatile tism_borrowed_shared_memory_t* shm, char* name);

/*
 * Close the borrowed shared memory and free its resources. This DOES NOT delete the allocation, the
 * allocation is freed by the operating system when all processes have closed their respective file
 * descriptors and TISM intentionally does not provide an API for destroying as instance of shared
 * memory.
 */
tism_result_t tism_owned_close(volatile tism_owned_shared_memory_t* shm);

/*
 * Close the owned shared memory and free its resources. This DOES NOT delete the allocation, the
 * allocation is freed by the operating system when all processes have closed their respective file
 * descriptors and TISM intentionally does not provide an API for destroying as instance of shared
 * memory.
 */
tism_result_t tism_borrowed_close(volatile tism_borrowed_shared_memory_t* shm);


/*
 * Write to the shared memory allocation by cloning the given data. This function will acquire and
 * release a write lock.
 */
tism_result_t tism_owned_write(volatile tism_owned_shared_memory_t* shm, const void* data);

/*
 * Read from the shared memory allocation by cloning the data into the given pointer. This function
 * will acquire and release a read lock.
 */
tism_result_t tism_owned_read(volatile tism_owned_shared_memory_t* shm, void* data);


/*
 * Read from the shared memory allocation by cloning the data into the given pointer. This function
 * will acquire and release a read lock.
 */
tism_result_t tism_borrowed_read(volatile tism_borrowed_shared_memory_t* shm, void* data);


/*
 * Lock the shared memory for writing. This is an exclusive lock and no other process will have
 * access while the memory is write locked. Be sure to unlock, ideally as soon as possible, after
 * calling this function.
 */
tism_result_t tism_unsafe_owned_write_lock(volatile tism_owned_shared_memory_t* shm, void** data);

/*
 * Lock the shared memory for reading. This lock allows any number of other readers but no writers
 * to access the lock. Be sure to unlock, ideally as soon as possible, after calling this function.
 */
tism_result_t tism_unsafe_owned_read_lock(volatile tism_owned_shared_memory_t* shm, void** data);

/*
 * Release the held lock. Sets the pointed to pointer to `NULL` assuming that it itself is not
 * `NULL`.
 */
tism_result_t tism_unsafe_owned_unlock(volatile tism_owned_shared_memory_t* shm, void** data);


/*
 * Lock the shared memory for reading. This lock allows any number of other readers but no writers
 * to access the lock. Be sure to unlock, ideally as soon as possible, after calling this function.
 */
tism_result_t tism_unsafe_borrowed_read_lock(volatile tism_borrowed_shared_memory_t* shm, void** data);

/*
 * Release the held lock. Sets the pointed to pointer to `NULL` assuming that it itself is not
 * `NULL`.
 */
tism_result_t tism_unsafe_borrowed_unlock(volatile tism_borrowed_shared_memory_t* shm, void** data);


/*
 * Lock for writing, clone the given data into the allocation, and unlock.
 */
tism_result_t _tism_write(volatile struct _tism_shared_memory* shm, const void* data);

/*
 * Lock for reading, clone the allocation into the given pointer, and unlock.
 */
tism_result_t _tism_read(volatile struct _tism_shared_memory* shm, void* data);


/*
 * Lock the shared memory for writing. This is an exclusive lock and no other process will have
 * access while the memory is write locked.
 */
tism_result_t _tism_write_lock(volatile struct _tism_shared_memory* shm);

/*
 * Lock the shared memory for reading. This lock allows any number of other readers but no writers
 * to access the lock.
 */
tism_result_t _tism_read_lock(volatile struct _tism_shared_memory* shm);

/*
 * Release the held lock.
 */
tism_result_t _tism_unlock(volatile struct _tism_shared_memory* shm);

/*
 * Close and free resources for this processes handle on the shared memory.
 */
tism_result_t _tism_close(volatile struct _tism_shared_memory* shm);

#endif  /* _TISM_H */
