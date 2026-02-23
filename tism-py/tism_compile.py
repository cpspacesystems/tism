# This module requires the following packages:
#  - cffi
#    - setuptools

from cffi import FFI
import itertools

TISM_HEADER_PATH: str = "tism.h"
TISM_SOURCE_PATH: str = "../tism-c/tism.c"

def remove_comments(source: str) -> str:
    """
    Remove comments from the given source, CFFI cannot parse comments.
    """

    MULTILINE_COMMENT_START: str = "/*"
    MULTILINE_COMMENT_END: str = "*/"
    SINGLE_LINE_COMMEND_START: str = "//"

    while MULTILINE_COMMENT_START in source and MULTILINE_COMMENT_END in source:
        start_idx = source.find(MULTILINE_COMMENT_START)
        end_idx = source.find(MULTILINE_COMMENT_END) + len(MULTILINE_COMMENT_END)

        source = source[0:start_idx] + source[end_idx:len(source)]

    while SINGLE_LINE_COMMEND_START in source:
        start_idx = source.find(SINGLE_LINE_COMMEND_START)
        end_idx = source.find("\n") + len("\n")

        source = source[0:start_idx] + source[end_idx:len(source)]

    return source

# Process our header a bit to make it usable for CFFI.
header_text = remove_comments(open(TISM_HEADER_PATH).read())
header_text = '\n'.join(
    filter(
        lambda s: s != "" and not s.startswith("#"),  # remove directives
        header_text.splitlines()
    )
)

# Compile our C code.
ffi = FFI()
ffi.cdef(header_text)
ffi.set_source("_tism", "#include \"" + TISM_HEADER_PATH + "\"\n", sources=[TISM_SOURCE_PATH], libraries=[])
ffi.compile()
