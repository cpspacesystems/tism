#!/usr/bin/env bash

gcc tests.c scrutiny/scrutiny.c tism.c -o tests
# use '-D TISM_DEBUG' to get messages about locking
./tests
