#!/usr/bin/env bash

gcc tism.c tests.c scrutiny/scrutiny.c -Iscrutiny/scrutiny.h -Itism.h -o tests
./tests
