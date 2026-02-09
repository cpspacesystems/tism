// Copyright (c) 2023 Evan Overman (https://an-prata.it). Licensed under the MIT License.
// See LICENSE file in repository root for complete license text.

/*
 * scrutiny.h
 *
 * You may define the SCRUTINY_TEST macro to make assertions behave in a testing
 * manor. Without this definition they will behave like the standard library's
 * `assert()` during the program's runtime.
 */

#ifndef SCRUTINY_H
#define SCRUTINY_H

#include <stdbool.h>
#include <time.h>
#include <sys/time.h>

#define TEXT_NORMAL "\033[0m"
#define TEXT_BOLD "\033[1m"
#define TEXT_ITALIC "\033[3m"
#define TEXT_GREEN "\033[0;32m"
#define TEXT_BLUE "\033[0;34m"
#define TEXT_RED "\033[0;31m"

#define SCRUTINY_UNIT_TEST(name) void name(scrutiny_test_run_t* _scrutiny_test_run)
#define SCRUTINY_BENCHMARK(name) const char* name(scrutiny_bench_run_t* _scrutiny_bench_run)

/*
 * This macro may be used to overwrite the start time of the current benchmark. 
 * If not used the time that the benchmark function is called will be used as 
 * the start time.
 */
#define scrutiny_bench_start() _scrutiny_bench_run->current_start = clock()

/*
 * This macro may be used to set the end time of a benchmark run before the
 * benchmark function returns. If not used the return time of the function will
 * be considered the benchmark's completion time.
 */
#define scrutiny_bench_end() _scrutiny_bench_run->current_end = clock()

/*
 * This macro must be used at the end of a benchmark in order to set the 
 * benchmark's name.
 */
#define scrutiny_bench_return() \
_scrutiny_bench_run->current_function = __func__; \
return TEXT_ITALIC __FILE__ TEXT_NORMAL ": " 

#ifdef SCRUTINY_DEBUG
	#define scrutiny_assert(a) \
	_scrutiny_assert(a, __FILE__, __func__, __LINE__)
	#define scrutiny_assert_equal(a, b) \
	_scrutiny_assert_equal(a, b, __FILE__, __func__, __LINE__)
	#define scrutiny_assert_not_equal(a, b) \
	_scrutiny_assert_not_equal(a, b, __FILE__, __func__, __LINE__)
	#define scrutiny_assert_greater_than(a, b) \
	_scrutiny_assert_greater_than(a, b, __FILE__, __func__, __LINE__)
	#define scrutiny_assert_less_than(a, b) \
	_scrutiny_assert_less_than(a, b, __FILE__, __func__, __LINE__)
	#define scrutiny_assert_greater_than_or_equal(a, b) \
	_scrutiny_assert_greater_than_or_equal(a, b, "scrutiny_assert_greater_than_or_equal", __FILE__, __func__, __LINE__)
	#define scrutiny_assert_less_than_or_equal(a, b) \
	_scrutiny_assert_less_than_or_equal(a, b, "scrutiny_assert_less_thanor_equal", __FILE__, __func__, __LINE__)
	#define scrutiny_assert_no_less_than(a, b) \
	_scrutiny_assert_greater_than_or_equal(a, b, "scrutiny_assert_no_less_than", __FILE__, __func__, __LINE__)
	#define scrutiny_assert_no_greater_than(a, b) \
	_scrutiny_assert_less_than_or_equal(a, b, "scrutiny_assert_no_greater_than", __FILE__, __func__, __LINE__)
#else
	#define scrutiny_assert(a) \
	_scrutiny_runtime_assert(a, __FILE__, __func__, __LINE__)
	#define scrutiny_assert_equal(a, b) \
	_scrutiny_runtime_assert_equal(a, b, __FILE__, __func__, __LINE__)
	#define scrutiny_assert_not_equal(a, b) \
	_scrutiny_runtime_assert_not_equal(a, b, __FILE__, __func__, __LINE__)
	#define scrutiny_assert_greater_than(a, b) \
	_scrutiny_runtime_assert_greater_than(a, b, __FILE__, __func__, __LINE__)
	#define scrutiny_assert_less_than(a, b) \
	_scrutiny_runtime_assert_less_than(a, b, __FILE__, __func__, __LINE__)
	#define scrutiny_assert_greater_than_or_equal(a, b) \
	_scrutiny_runtime_assert_greater_than_or_equal(a, b, "scrutiny_assert_greater_than_or_equal", __FILE__, __func__, __LINE__)
	#define scrutiny_assert_less_than_or_equal(a, b) \
	_scrutiny_runtime_assert_less_than_or_equal(a, b, "scrutiny_assert_less_thanor_equal", __FILE__, __func__, __LINE__)
	#define scrutiny_assert_no_less_than(a, b) \
	_scrutiny_runtime_assert_greater_than_or_equal(a, b, "scrutiny_assert_no_less_than", __FILE__, __func__, __LINE__)
	#define scrutiny_assert_no_greater_than(a, b) \
	_scrutiny_runtime_assert_less_than_or_equal(a, b, "scrutiny_assert_no_greater_than", __FILE__, __func__, __LINE__)
#endif // SCRUTINY_DEBUG

#define _scrutiny_assert(a, file, func, line) \
if (!_scrutiny_record_assert(_scrutiny_test_run, (a), "scrutiny_assert", #a, file, func, line)) \
	return
#define _scrutiny_assert_equal(a, b, file, func, line) \
if (!_scrutiny_record_assert(_scrutiny_test_run, (a) == (b), "scrutiny_assert_equal", #a ", " #b, file, func, line)) \
	return
#define _scrutiny_assert_not_equal(a, b, file, func, line) \
if (!_scrutiny_record_assert(_scrutiny_test_run, (a) != (b), "scrutiny_assert_not_equal", #a ", " #b, file, func, line)) \
	return
#define _scrutiny_assert_greater_than(a, b, file, func, line) \
if (!_scrutiny_record_assert(_scrutiny_test_run, (a) > (b), "scrutiny_assert_greater_than", #a ", " #b, file, func, line)) \
	return
#define _scrutiny_assert_less_than(a, b, file, func, line) \
if (!_scrutiny_record_assert(_scrutiny_test_run, (a) < (b), "scrutiny_assert_less_than", #a ", " #b, file, func, line)) \
	return
#define _scrutiny_assert_greater_than_or_equal(a, b, assert, file, func, line) \
if (!_scrutiny_record_assert(_scrutiny_test_run, (a) >= (b), assert, #a ", " #b, file, func, line)) \
	return
#define _scrutiny_assert_less_than_or_equal(a, b, assert, file, func, line) \
if (!_scrutiny_record_assert(_scrutiny_test_run, (a) <= (b), assert, #a ", " #b, file, func, line)) \
	return

#define _scrutiny_runtime_assert(a, file, func, line) \
_scrutiny_abort_assert((a), "scrutiny_assert", #a, file, func, line)
#define _scrutiny_runtime_assert_equal(a, b, file, func, line) \
_scrutiny_abort_assert((a) == (b), "scrutiny_assert_equal", #a ", " #b, file, func, line)
#define _scrutiny_runtime_assert_not_equal(a, b, file, func, line) \
_scrutiny_abort_assert((a) != (b), "scrutiny_assert_not_equal", #a ", " #b, file, func, line)
#define _scrutiny_runtime_assert_greater_than(a, b, file, func, line) \
_scrutiny_abort_assert((a) > (b), "scrutiny_assert_greater_than", #a ", " #b, file, func, line)
#define _scrutiny_runtime_assert_less_than(a, b, file, func, line) \
_scrutiny_abort_assert((a) < (b), "scrutiny_assert_less_than", #a ", " #b, file, func, line)
#define _scrutiny_runtime_assert_greater_than_or_equal(a, b, assert, file, func, line) \
_scrutiny_abort_assert((a) >= (b), assert, #a ", " #b, file, func, line)
#define _scrutiny_runtime_assert_less_than_or_equal(a, b, assert, file, func, line) \
_scrutiny_abort_assert((a) <= (b), assert, #a ", " #b, file, func, line)

typedef enum {
	SCRUTINY_SUCCESS = true,
	SCRUTINY_FAILURE = false,
} scrutiny_test_result_t;

typedef struct {
	const char* previous_function;
	scrutiny_test_result_t current_test_result;
	unsigned long asserts_passed;
	unsigned long asserts_failed;
	unsigned long tests_passed;
	unsigned long tests_failed;
} scrutiny_test_run_t;

typedef struct {
	const char* current_function;
	clock_t current_proc_start;
	clock_t current_proc_end;
	struct timeval current_start;
	struct timeval current_end;
} scrutiny_bench_run_t;

typedef void (*scrutiny_test_t)(scrutiny_test_run_t*);
typedef const char* (*scrutiny_benchmark_t)(scrutiny_bench_run_t*);

/*
 * This function expects that `tests` points to a NULL terminated array of
 * `scrutiny_test_t` function pointers.
 */
void scrutiny_run_tests(scrutiny_test_t* tests);
void scrutiny_run_tests_with_stats(scrutiny_test_t* tests);

/*
 * This function expects that `benchmarks` points to a NULL terminated array of
 * `scrutiny_benchmark_t` function pointers. This function will print both the 
 * time for execution and the CPU time spent running the benchmark. Usually 
 * these values will be nearly equal except in the event they call on I/O or
 * functions like `sleep()`.
 */
void scrutiny_run_benchmarks(scrutiny_benchmark_t* benchmarks, unsigned int passes);

bool _scrutiny_abort_assert(bool succeeded, const char* assert, const char* condition, const char* file, const char* function, unsigned long line);

bool _scrutiny_record_assert(scrutiny_test_run_t* test_run, bool succeeded, const char* assert, const char* condition, const char* file, const char* function, unsigned long line);

#endif // SCRUTINY_H

