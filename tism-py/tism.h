#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>
#include <unistd.h>

typedef struct _tism_shared_memory tism_owned_shared_memory_t;
typedef struct _tism_shared_memory tism_borrowed_shared_memory_t;

struct _tism_shared_memory {
	int fd;
	uint64_t last_read_count;
	struct _tism_allocation* allocation;
	char extra[8 * 4];
};

struct _tism_allocation {
	size_t data_size;
	uint8_t major_version;
	uint8_t minor_version;
	uint16_t patch_version;
	_Atomic uint64_t total_writes;
	_Atomic bool is_zombie;
	char data[];
};

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
    TISM_DOES_NOT_EXIST,   /* An allocation by the given name does not exist. */

	TISM_UNKNOWN,  /* An unknown error occured. */
} tism_result_t;

tism_result_t tism_create(volatile tism_owned_shared_memory_t* shm, char* name, const void* data, size_t n);
tism_result_t tism_open(volatile tism_borrowed_shared_memory_t* shm, char* name);
tism_result_t tism_wait_and_open(volatile tism_borrowed_shared_memory_t* shm, char* name);
tism_result_t tism_owned_close(volatile tism_owned_shared_memory_t* shm);
tism_result_t tism_borrowed_close(volatile tism_borrowed_shared_memory_t* shm);

tism_result_t tism_owned_write(volatile tism_owned_shared_memory_t* shm, const void* data);
tism_result_t tism_owned_read(volatile tism_owned_shared_memory_t* shm, void* data);
uint64_t tism_owned_get_total_writes(tism_owned_shared_memory_t* shm);

tism_result_t tism_borrowed_read(volatile tism_borrowed_shared_memory_t* shm, void* data);
bool tism_borrowed_has_changed(tism_borrowed_shared_memory_t* shm);
int64_t tism_borrowed_staleness(tism_borrowed_shared_memory_t* shm);
uint64_t tism_borrowed_get_total_writes(tism_borrowed_shared_memory_t* shm);

tism_result_t tism_unsafe_owned_write_lock(volatile tism_owned_shared_memory_t* shm, void** data);
tism_result_t tism_unsafe_owned_read_lock(volatile tism_owned_shared_memory_t* shm, void** data);
tism_result_t tism_unsafe_owned_unlock(volatile tism_owned_shared_memory_t* shm, void** data);

tism_result_t tism_unsafe_borrowed_read_lock(volatile tism_borrowed_shared_memory_t* shm, void** data);
tism_result_t tism_unsafe_borrowed_unlock(volatile tism_borrowed_shared_memory_t* shm, void** data);

tism_result_t _tism_write(volatile struct _tism_shared_memory* shm, const void* data);
tism_result_t _tism_read(volatile struct _tism_shared_memory* shm, void* data);
bool _tism_has_changed(tism_borrowed_shared_memory_t* shm);
int64_t _tism_staleness(tism_borrowed_shared_memory_t* shm);
uint64_t _tism_get_total_writes(struct _tism_shared_memory* shm);

tism_result_t _tism_write_lock(volatile struct _tism_shared_memory* shm);
tism_result_t _tism_read_lock(volatile struct _tism_shared_memory* shm);
tism_result_t _tism_unlock(volatile struct _tism_shared_memory* shm);
tism_result_t _tism_close(volatile struct _tism_shared_memory* shm);
