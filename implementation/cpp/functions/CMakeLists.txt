add_library( frost-functions
    closure.cpp
    lambda.cpp
    builtins/and_then.cpp
    builtins/debug-helpers.cpp
    builtins/error-handling.cpp
    builtins/filesystem.cpp
    builtins/free-operators.cpp
    builtins/json.cpp
    builtins/math.cpp
    builtins/mutable-cell.cpp
    builtins/os.cpp
    builtins/output.cpp
    builtins/pack_call.cpp
    builtins/ranges.cpp
    builtins/regex.cpp
    builtins/streams.cpp
    builtins/string-ops.cpp
    builtins/structure-ops.cpp
    builtins/type-conversions.cpp
    builtins/type-tests.cpp
)

if(WITH_HTTP)
    target_sources( frost-functions
    PRIVATE
    builtins/http.cpp
    builtins/async-impl/http/request.cpp
    )
endif()

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
    frost-symbol-table
    Boost::regex
    Boost::json
    cppcodec::cppcodec
)

if(WITH_HTTP)
    target_link_libraries( frost-functions
        PUBLIC
        Boost::url
        OpenSSL::SSL
        OpenSSL::Crypto
    )
endif()

set(FUNCTIONS_TEST_FILES
    builtin
    closure
    lambda
)

set(FUNCTIONS_BUILTINS_TEST_FILES
    keys
    and-then
    type-tests
    type-conversions
    values
    id
    len
    range
    nulls
    ranges
    pack_call
    debug-helpers
    mformat
    hard-to-test
    error-handling
    free-operators
    math
    string-ops
    regex
    mutable-cell
    streams
    json
    base64
    filesystem
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
