#!/usr/bin/env bash

gcc tism.c tests.c scrutiny/scrutiny.c -o tests
./tests
