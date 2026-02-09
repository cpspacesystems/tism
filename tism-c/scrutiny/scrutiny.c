// Copyright (c) 2023 Evan Overman (https://an-prata.it). Licensed under the MIT License.
// See LICENSE file in repository root for complete license text.

#include <stdio.h>
#include <stdlib.h>
#include <sys/time.h>
#include "scrutiny.h"

static void _scrutiny_run_tests(scrutiny_test_t* tests, bool with_stats) {
	scrutiny_test_run_t test_run = {
		.previous_function = NULL,
		.current_test_result = SCRUTINY_SUCCESS,
		
		.asserts_passed = 0,
		.asserts_failed = 0,
		
		.tests_passed = 0,
		.tests_failed = 0,
	};
	
	for (unsigned int i = 0; tests[i]; i++) {
		test_run.current_test_result = SCRUTINY_SUCCESS;
		tests[i](&test_run);
		
		if (test_run.current_test_result == SCRUTINY_FAILURE) {
			test_run.tests_failed++;
			printf("(" TEXT_RED "x" TEXT_NORMAL ") test failed: " TEXT_BLUE TEXT_ITALIC "%s\n" TEXT_NORMAL, test_run.previous_function);
		} else {
			test_run.tests_passed++;
			printf("(" TEXT_GREEN "✓" TEXT_NORMAL ") test passed: " TEXT_BLUE TEXT_ITALIC "%s\n" TEXT_NORMAL, test_run.previous_function);
		}
	}

	if (!with_stats) {
		if (test_run.asserts_failed > 0) {
			exit(EXIT_FAILURE);
		}

		exit(EXIT_SUCCESS);
	}

	double percent_tests_passed = 100.0 * (double)test_run.tests_passed / (double)(test_run.tests_failed + test_run.tests_passed);
	double percent_asserts_passed = 100.0 * (double)test_run.asserts_passed / (double)(test_run.asserts_failed + test_run.asserts_passed);

	printf("\n\nscrutiny made %zu assertions from %zu tests\n", test_run.asserts_failed + test_run.asserts_passed, test_run.tests_failed + test_run.tests_passed);

	if (test_run.asserts_passed > 0) {
		printf("\n(" TEXT_GREEN "✓" TEXT_NORMAL ") %zu of %zu tests passed (%2.1f%%)\n", test_run.tests_passed, test_run.tests_failed + test_run.tests_passed, percent_tests_passed);
        printf("(" TEXT_GREEN "✓" TEXT_NORMAL ") %zu of %zu assertions passed (%2.1f%%)\n", test_run.asserts_passed, test_run.asserts_failed + test_run.asserts_passed, percent_asserts_passed);
	}

	if (test_run.asserts_failed > 0) {
		printf("\n(" TEXT_RED "x" TEXT_NORMAL ") %zu of %zu tests failed (%2.1f%%)\n", test_run.tests_failed, test_run.tests_failed + test_run.tests_passed, 100.0 - percent_tests_passed);
        printf("(" TEXT_RED "x" TEXT_NORMAL ") %zu of %zu assertions failed (%2.1f%%)\n", test_run.asserts_failed, test_run.asserts_failed + test_run.asserts_passed, 100.0 - percent_asserts_passed);
		exit(EXIT_FAILURE);
	}

	exit(EXIT_SUCCESS);
}

void scrutiny_run_tests(scrutiny_test_t* tests) {
	_scrutiny_run_tests(tests, false);
}

void scrutiny_run_tests_with_stats(scrutiny_test_t* tests) {
	_scrutiny_run_tests(tests, true);
}

void scrutiny_run_benchmarks(scrutiny_benchmark_t* benchmarks, unsigned int passes) {
	scrutiny_bench_run_t bench_run = {
		.current_function = NULL,
		.current_proc_start = -1,
		.current_proc_end = -1,
	};

	for (unsigned int i = 0; benchmarks[i]; i++) {
		const char* name_prefix;
		double cpu_total = 0;
		double total = 0;
		
		for (unsigned int j = 0; j < passes; j++) {
			bench_run.current_proc_end = -1;
			bench_run.current_proc_start = clock();
			gettimeofday(&bench_run.current_start, NULL);
		
			name_prefix = benchmarks[i](&bench_run);
		
			clock_t return_time = clock();
			if (bench_run.current_proc_end < 0) {
				gettimeofday(&bench_run.current_end, NULL);
				bench_run.current_proc_end = return_time;
			}

			cpu_total += (double)(bench_run.current_proc_end - bench_run.current_proc_start) / CLOCKS_PER_SEC;
			total += (double)(bench_run.current_end.tv_sec + bench_run.current_end.tv_usec / 1e+6) - (double)(bench_run.current_start.tv_sec + bench_run.current_start.tv_usec / 1e+6);
		}

		cpu_total /= passes;
		total /= passes;
		
		printf("%s" TEXT_BLUE TEXT_BOLD "%s " TEXT_NORMAL "- " TEXT_GREEN "%f seconds (%f seconds CPU time)" TEXT_NORMAL "\n", name_prefix, bench_run.current_function, total, cpu_total);
	}
}

bool _scrutiny_abort_assert(bool succeeded, const char* assert, const char* condition, const char* file, const char* function, unsigned long line) {
	if (!succeeded) {
		printf(TEXT_RED TEXT_BOLD "ASSERT FAILED " TEXT_NORMAL TEXT_ITALIC "%s(%s) " TEXT_NORMAL ": " TEXT_ITALIC "line %zu " TEXT_NORMAL "of " TEXT_ITALIC "%s " TEXT_NORMAL "in " TEXT_BLUE TEXT_ITALIC "%s\n" TEXT_NORMAL, assert, condition, line, file, function);
		exit(EXIT_FAILURE);
	}

	return succeeded;
}

bool _scrutiny_record_assert(scrutiny_test_run_t* test_run, bool succeeded, const char* assert, const char* condition, const char* file, const char* function, unsigned long line) {
	if (test_run->previous_function != function) {
		if (!succeeded && test_run->previous_function) {
			printf("\n");
		}

		test_run->previous_function = function;
	}

	if (!succeeded) {
		test_run->current_test_result = SCRUTINY_FAILURE;
		test_run->asserts_failed++;
		printf(TEXT_RED TEXT_BOLD "ASSERT FAILED " TEXT_NORMAL TEXT_ITALIC "%s(%s) " TEXT_NORMAL ": " TEXT_ITALIC "line %zu " TEXT_NORMAL "of " TEXT_ITALIC "%s " TEXT_NORMAL "in " TEXT_BLUE TEXT_ITALIC "%s\n" TEXT_NORMAL, assert, condition, line, file, function);
	} else {
		test_run->asserts_passed++;
	}

	return succeeded;
}
