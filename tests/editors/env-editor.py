#!/usr/bin/env python3

"""
A mock editor that reads the $TEST_EDITOR_INPUT and $TEST_EDITOR_OUTPUT
environment variables, copies the edited file to $TEST_EDITOR_INPUT and copies
$TEST_EDITOR_OUTPUT to the edited file.
"""

import os
import shutil
import sys


ENV_INPUT = 'TEST_EDITOR_INPUT'
ENV_OUTPUT = 'TEST_EDITOR_OUTPUT'


def require_env_var(name):
    if not name in os.environ:
        print("Environment variable {} not set, aborting.".format(name))
        sys.exit(1)
    return os.environ[name]


in_file_name = require_env_var(ENV_INPUT)
out_file_name = require_env_var(ENV_OUTPUT)

if len(sys.argv) != 2:
    print("Expected exactly one argument, aborting.")
    sys.exit(1)
edited_file_name = sys.argv[1]

shutil.copy(edited_file_name, in_file_name)
shutil.copy(out_file_name, edited_file_name)
