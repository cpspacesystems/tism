#define SCRUTINY_DEBUG

#include "tism.h"
#include "scrutiny/scrutiny.h"

SCRUTINY_UNIT_TEST(test_create) {
	int init_data = 0;
	tism_owned_shared_memory_t owner;
	scrutiny_assert_equal(TISM_OK, tism_create(&owner, "test_create_shm", &init_data, sizeof init_data));

	int read_data;
	scrutiny_assert_equal(TISM_OK, tism_borrowed_read(&owner, &read_data));
	scrutiny_assert_equal(read_data, init_data);

	init_data = 300;
	scrutiny_assert_equal(TISM_OK, tism_owned_write(&owner, &init_data));

	scrutiny_assert_equal(TISM_OK, tism_owned_close(&owner));
}

SCRUTINY_UNIT_TEST(test_open) {
	int init_data = 0;
	tism_owned_shared_memory_t owner;
	scrutiny_assert_equal(TISM_OK, tism_create(&owner, "test_open_shm", &init_data, sizeof init_data));

	tism_borrowed_shared_memory_t borrower;
	scrutiny_assert_equal(TISM_OK, tism_open(&borrower, "test_open_shm", sizeof init_data));

	int read_data;
	scrutiny_assert_equal(TISM_OK, tism_borrowed_read(&borrower, &read_data));
	scrutiny_assert_equal(read_data, init_data);

	init_data = 300;
	scrutiny_assert_equal(TISM_OK, tism_owned_write(&owner, &init_data));

	scrutiny_assert_equal(TISM_OK, tism_borrowed_close(&borrower));
	scrutiny_assert_equal(TISM_OK, tism_owned_close(&owner));
}

SCRUTINY_UNIT_TEST(test_unsafe_read) {
	int init_data = 0;
	tism_owned_shared_memory_t owner;
	scrutiny_assert_equal(TISM_OK, tism_create(&owner, "test_unsafe_read_shm", &init_data, sizeof init_data));

	tism_borrowed_shared_memory_t borrower;
	scrutiny_assert_equal(TISM_OK, tism_open(&borrower, "test_unsafe_read_shm", sizeof init_data));

	int* shm_data = NULL;
	scrutiny_assert_equal(TISM_OK, tism_unsafe_borrowed_read_lock(&borrower, (void**)&shm_data));
	scrutiny_assert(shm_data);
	scrutiny_assert_equal(0, *shm_data);
	scrutiny_assert_equal(TISM_OK, tism_unsafe_borrowed_unlock(&borrower, (void**)&shm_data));
	scrutiny_assert(!shm_data);

	init_data = 803;
	scrutiny_assert_equal(TISM_OK, tism_owned_write(&owner, &init_data));

	scrutiny_assert_equal(TISM_OK, tism_unsafe_borrowed_read_lock(&borrower, (void**)&shm_data));
	scrutiny_assert(shm_data);
	scrutiny_assert_equal(803, *shm_data);
	scrutiny_assert_equal(TISM_OK, tism_unsafe_borrowed_unlock(&borrower, (void**)&shm_data));
	scrutiny_assert(!shm_data);

	scrutiny_assert_equal(TISM_OK, tism_borrowed_close(&borrower));
	scrutiny_assert_equal(TISM_OK, tism_owned_close(&owner));
}

SCRUTINY_UNIT_TEST(test_unsafe_write) {
	int init_data = 0;
	tism_owned_shared_memory_t owner;
	scrutiny_assert_equal(TISM_OK, tism_create(&owner, "test_unsafe_write_shm", &init_data, sizeof init_data));

	tism_borrowed_shared_memory_t borrower;
	scrutiny_assert_equal(TISM_OK, tism_open(&borrower, "test_unsafe_write_shm", sizeof init_data));

	int* shm_data = NULL;
	scrutiny_assert_equal(TISM_OK, tism_unsafe_owned_write_lock(&owner, (void**)&shm_data));
	scrutiny_assert(shm_data);
	*shm_data = 920;
	scrutiny_assert_equal(TISM_OK, tism_unsafe_owned_unlock(&owner, (void**)&shm_data));
	scrutiny_assert(!shm_data);
	
	int read_data;
	scrutiny_assert_equal(TISM_OK, tism_borrowed_read(&borrower, &read_data));
	scrutiny_assert_equal(920, read_data);

	scrutiny_assert_equal(TISM_OK, tism_borrowed_close(&borrower));
	scrutiny_assert_equal(TISM_OK, tism_owned_close(&owner));
}

int main() {
	scrutiny_test_t tests[] = {
		test_create,
		test_open,
		test_unsafe_read,
		test_unsafe_write,
		NULL
	};

	scrutiny_run_tests_with_stats(tests);
}
