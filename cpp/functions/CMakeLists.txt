add_library( frost-functions
    builtin.cpp
    closure.cpp
    lambda.cpp
    stdlib.cpp
    builtins/and-then.cpp
    builtins/debug-helpers.cpp
    builtins/error-handling.cpp
    builtins/free-operators.cpp
    builtins/mutable-cell.cpp
    builtins/output.cpp
    builtins/call.cpp
    builtins/ranges.cpp
    builtins/streams.cpp
    builtins/string-ops.cpp
    builtins/structure-ops.cpp
    builtins/type-conversions.cpp
    builtins/type-tests.cpp
    stdlib/encoding.cpp
    stdlib/fs.cpp
    stdlib/hash.cpp
    stdlib/io.cpp
    stdlib/json.cpp
    stdlib/math.cpp
    stdlib/os.cpp
    stdlib/regex.cpp
)

target_include_directories( frost-functions
    PUBLIC
    include/
)

target_link_libraries( frost-functions
    PUBLIC
    frost-common
    frost-utils
    frost-ast
    frost-value
    frost-execution-context
    Boost::regex
    Boost::json
    Boost::url
    OpenSSL::Crypto
    ZLIB::ZLIB
    cppcodec::cppcodec
)

set(FUNCTIONS_TEST_FILES
    builtin
    closure
    lambda
    stdlib
)

set(FUNCTIONS_BUILTINS_TEST_FILES
    keys
    has
    includes
    and-then
    type-tests
    type-conversions
    values
    id
    len
    range
    nulls
    ranges
    call
    debug-helpers
    mformat
    hard-to-test
    error-handling
    free-operators
    string-ops
    mutable-cell
    streams
)

set(FUNCTIONS_STDLIB_TEST_FILES
    encoding
    fs
    hash
    io
    json
    math
    os
    regex
)

foreach(test_file IN LISTS FUNCTIONS_TEST_FILES)
    make_test( tests/${test_file}.cpp
        LIBS
        frost-functions
    )
endforeach()

foreach(test_file IN LISTS FUNCTIONS_BUILTINS_TEST_FILES)
    make_test( tests/builtins/${test_file}.cpp
        LIBS
        frost-functions
    )
endforeach()

foreach(test_file IN LISTS FUNCTIONS_STDLIB_TEST_FILES)
    make_test( tests/stdlib/${test_file}.cpp
        LIBS
        frost-functions
    )
endforeach()
