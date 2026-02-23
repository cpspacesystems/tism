#include "tism.h"

#include <errno.h>
#include <fcntl.h>
#include <stddef.h>
#include <string.h>
#include <unistd.h>

#include <stdio.h>
#define TISM_OVERHEAD (sizeof(struct _tism_allocation) - 1)

#define CREATE_FLAGS (O_CREAT | O_RDWR | O_TRUNC | O_EXCL)
#define CREATE_MODE  (S_IRUSR | S_IWUSR | S_IRGRP | S_IWGRP | S_IROTH)

tism_result_t tism_create(volatile tism_owned_shared_memory_t* shm, char* name, const void* data, size_t n) {
	if (strlen(name) > TISM_MAX_NAME_LENGTH) {
		return TISM_INVALID_ARGUMENT;
	}

	/*
	 * Naming requirements are subtly different between Unix and Linux, we account for this here
	 * automatically in order to make TISM portable.
	 */

#if defined(__APPLE__)
	char nonportable_name[TISM_MAX_NAME_LENGTH + 2];  /* Space for a slash and null character. */
	nonportable_name[0] = '/';
	memcpy(&nonportable_name[1], name, strlen(name));
#elif defined(__linux__)
	char* nonportable_name = name;
#else
#error "Non Unix or Linux systems no supported"
#endif

	/*
	 * This cleans up allocations which were not properly cleaned up before, as a result we allow the
	 * `ENOENT` (no entry, i.e. does not exist) error because we don't expect the allocation to exist
	 * or not at this state, the purpose of this call is specifically to ensure the allocation does not
	 * yet exist, so in our case the `ENOENT` error is more succesfull than if `shm_unlink` returned
	 * zero.
	 */
	 
	if (shm_unlink(nonportable_name) != 0 && errno != ENOENT) {
		switch (errno) {
			case EACCES:       return TISM_BAD_PERMISSIONS;
			case ENAMETOOLONG: return TISM_INVALID_ARGUMENT;
			default:           return TISM_UNKNOWN;
		}
	}

	shm->fd = shm_open(nonportable_name, CREATE_FLAGS, CREATE_MODE);

	if (shm->fd < 0) {
		switch (errno) {
			case EACCES:       return TISM_BAD_PERMISSIONS;
			case EINTR:        return TISM_INTERUPTED;
			case EINVAL:       return TISM_UNSUPPORTED;
			case EMFILE:       return TISM_TOO_MANY_FDS;
			case ENAMETOOLONG: return TISM_INVALID_ARGUMENT;
			case ENFILE:       return TISM_FILE_TABLE;
			case ENOSPC:       return TISM_NO_SPACE;
			default:           return TISM_UNKNOWN;
		}
	}

	if (ftruncate(shm->fd, TISM_OVERHEAD + n) < 0) {
		switch (errno) {
			case EFBIG:        return TISM_TOO_BIG;
			case EINVAL:
			case EPERM:
			case EROFS:
			case EACCES:
			case EFAULT:       return TISM_BAD_PERMISSIONS;
			case ENAMETOOLONG: return TISM_INVALID_ARGUMENT;
			default:           return TISM_UNKNOWN;
		}
	}

	volatile void* allocation = mmap(
		NULL,
		TISM_OVERHEAD + n,
		PROT_WRITE | PROT_READ,
		MAP_SHARED,
		shm->fd,
		0
	);

	if (allocation == MAP_FAILED) {
		switch (errno) {
			case EACCES:
			case EINVAL: return TISM_BAD_PERMISSIONS;
			default: 	 return TISM_UNKNOWN;
		}
	}

	shm->allocation = (struct _tism_allocation*)allocation;
	shm->allocation->data_size = n;
	shm->allocation->major_version = TISM_MAJOR_VERSION;
	shm->allocation->minor_version = TISM_MINOR_VERSION;
	shm->allocation->patch_version = TISM_PATCH_VERSION;
	
	switch (pthread_rwlock_init(&shm->allocation->rw_lock, NULL)) {
		case 0:     break;
		case EPERM: return TISM_BAD_PERMISSIONS;
		default:    return TISM_UNKNOWN;
	}

	memcpy(&shm->allocation->data, data, n);

	return TISM_OK;
}

tism_result_t tism_open(volatile tism_borrowed_shared_memory_t* shm, char* name) {
	if (strlen(name) > TISM_MAX_NAME_LENGTH) {
		return TISM_INVALID_ARGUMENT;
	}

	/*
	 * Naming requirements are subtly different between Unix and Linux, we account for this here
	 * automatically in order to make TISM portable.
	 */

#if defined(__APPLE__)
	char nonportable_name[TISM_MAX_NAME_LENGTH + 2];  /* Space for a slash and null character. */
	nonportable_name[0] = '/';
	memcpy(&nonportable_name[1], name, strlen(name));
#elif defined(__linux__)
	char* nonportable_name = name;
#else
#error "Non Unix or Linux systems no supported"
#endif

	/*
	 * Account for another subtle Unix/Linux API difference. In this case the Linux version probably
	 * works on Unix too, but whatever.
	 */
	 
#if defined(__APPLE__)
	shm->fd = shm_open(nonportable_name, O_RDWR);
#elif defined(__linux__)
	shm->fd = shm_open(nonportable_name, O_RDWR, 0);
#else
#error "Non Unix or Linux systems no supported"
#endif

	if (shm->fd < 0) {
		switch (errno) {
			case EACCES:       return TISM_BAD_PERMISSIONS;
			case EINTR:        return TISM_INTERUPTED;
			case EINVAL:       return TISM_UNSUPPORTED;
			case EMFILE:       return TISM_TOO_MANY_FDS;
			case ENAMETOOLONG: return TISM_INVALID_ARGUMENT;
			case ENFILE:       return TISM_FILE_TABLE;
			case ENOSPC:       return TISM_NO_SPACE;
			default:           return TISM_UNKNOWN;
		}
	}

	void* allocation = mmap(
		NULL,
		sizeof(struct _tism_allocation),
		PROT_WRITE | PROT_READ,
		MAP_SHARED,
		shm->fd,
		0
	);

	if (allocation == MAP_FAILED) {
		switch (errno) {
			case EACCES:
			case EINVAL: return TISM_BAD_PERMISSIONS;
			default: 	 return TISM_UNKNOWN;
		}
	}

	struct _tism_allocation* tism_allocation = (struct _tism_allocation*)allocation;
	size_t data_size = tism_allocation->data_size;

	if (tism_allocation->major_version != TISM_MAJOR_VERSION) {
		return TISM_VERSION_MISMATCH;
	}

	munmap(allocation, sizeof(struct _tism_allocation));

	allocation = mmap(
		NULL,
		TISM_OVERHEAD + data_size,
		PROT_WRITE | PROT_READ,
		MAP_SHARED,
		shm->fd,
		0
	);

	if (allocation == MAP_FAILED) {
		switch (errno) {
			case EACCES:
			case EINVAL: return TISM_BAD_PERMISSIONS;
			default: 	 return TISM_UNKNOWN;
		}
	}
 
	shm->allocation = (struct _tism_allocation*)allocation;

	return TISM_OK;
}

tism_result_t tism_owned_close(volatile tism_owned_shared_memory_t* shm) {
	return _tism_close((struct _tism_shared_memory*)shm);
}

tism_result_t tism_borrowed_close(volatile tism_borrowed_shared_memory_t* shm) {
	return _tism_close((struct _tism_shared_memory*)shm);
}
 

tism_result_t tism_owned_write(volatile tism_owned_shared_memory_t* shm, const void* data) {
	return _tism_write((struct _tism_shared_memory*)shm, data);
}

tism_result_t tism_owned_read(volatile tism_owned_shared_memory_t* shm, void* data) {
	return _tism_read((struct _tism_shared_memory*)shm, data);
}


tism_result_t tism_borrowed_read(volatile tism_borrowed_shared_memory_t* shm, void* data) {
	return _tism_read((struct _tism_shared_memory*)shm, data);
}


tism_result_t tism_unsafe_owned_write_lock(volatile tism_owned_shared_memory_t* shm, void** data) {
	TISM_MBIND(_tism_write_lock((struct _tism_shared_memory*)shm));
	*data = &shm->allocation->data;
	return TISM_OK;
}

tism_result_t tism_unsafe_owned_read_lock(volatile tism_owned_shared_memory_t* shm, void** data) {
	TISM_MBIND(_tism_read_lock((struct _tism_shared_memory*)shm));
	*data = &shm->allocation->data;
	return TISM_OK;
}

tism_result_t tism_unsafe_owned_unlock(volatile tism_owned_shared_memory_t* shm, void** data){
	if (data) {
		*data = NULL;
	}

	return _tism_unlock((struct _tism_shared_memory*)shm);
}


tism_result_t tism_unsafe_borrowed_read_lock(volatile tism_borrowed_shared_memory_t* shm, void** data) {
	TISM_MBIND(_tism_read_lock((struct _tism_shared_memory*)shm));
	*data = &shm->allocation->data;
	return TISM_OK;
}

tism_result_t tism_unsafe_borrowed_unlock(volatile tism_borrowed_shared_memory_t* shm, void** data) {
	if (data) {
		*data = NULL;
	}

	return _tism_unlock((struct _tism_shared_memory*)shm);
}


tism_result_t _tism_write(volatile struct _tism_shared_memory* shm, const void* data) {
	TISM_MBIND(_tism_write_lock(shm));
	memcpy(&shm->allocation->data, data, shm->allocation->data_size);
	TISM_MBIND(_tism_unlock(shm));

	return TISM_OK;
}

tism_result_t _tism_read(volatile struct _tism_shared_memory* shm, void* data) {
	TISM_MBIND(_tism_read_lock(shm));
	memcpy(data, &shm->allocation->data, shm->allocation->data_size);
	TISM_MBIND(_tism_unlock(shm));

	return TISM_OK;
}


tism_result_t _tism_write_lock(volatile struct _tism_shared_memory* shm) {
	switch (pthread_rwlock_wrlock(&shm->allocation->rw_lock)) {
		case 0:     return TISM_OK;
		default:    return TISM_UNKNOWN;
	}
}

tism_result_t _tism_read_lock(volatile struct _tism_shared_memory* shm) {
	switch (pthread_rwlock_rdlock(&shm->allocation->rw_lock)) {
		case 0:     return TISM_OK;
		default:    return TISM_UNKNOWN;
	}
}

tism_result_t _tism_unlock(volatile struct _tism_shared_memory* shm) {
	switch (pthread_rwlock_unlock(&shm->allocation->rw_lock)) {
		case 0:     return TISM_OK;
		case EPERM: return TISM_BAD_PERMISSIONS;
		default:    return TISM_UNKNOWN;
	}
}

tism_result_t _tism_close(volatile struct _tism_shared_memory* shm) {
	if (close(shm->fd) != 0) {
		switch (errno) {
			case EINTR: return TISM_INTERUPTED;
			default:    return TISM_UNKNOWN;
		}
	}

	if (munmap(shm->allocation, TISM_OVERHEAD + shm->allocation->data_size) != 0) {
		return TISM_UNKNOWN;
	}

	return TISM_OK;
}
