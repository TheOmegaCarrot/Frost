add_library( frost-functions
    builtins.cpp
    closure.cpp
    lambda.cpp
    builtins/and_then.cpp
    builtins/debug-helpers.cpp
    builtins/error-handling.cpp
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
)

macro(functions_test file)
    make_test( ${file}
        LIBS
        frost-functions
    )
endmacro()

functions_test(tests/builtin.cpp)

functions_test(tests/builtins/keys.cpp)

functions_test(tests/builtins/and-then.cpp)

functions_test(tests/builtins/type-tests.cpp)

functions_test(tests/builtins/type-conversions.cpp)

functions_test(tests/builtins/values.cpp)

functions_test(tests/builtins/len.cpp)

functions_test(tests/builtins/range.cpp)

functions_test(tests/builtins/nulls.cpp)

functions_test(tests/builtins/ranges.cpp)

functions_test(tests/builtins/pack_call.cpp)

functions_test(tests/builtins/debug-helpers.cpp)

functions_test(tests/builtins/mformat.cpp)

functions_test(tests/builtins/hard-to-test.cpp)

functions_test(tests/closure.cpp)

functions_test(tests/lambda.cpp)

functions_test(tests/builtins/error-handling.cpp)

functions_test(tests/builtins/free-operators.cpp)

functions_test(tests/builtins/math.cpp)

functions_test(tests/builtins/string-ops.cpp)

functions_test(tests/builtins/regex.cpp)

functions_test(tests/builtins/mutable-cell.cpp)

functions_test(tests/builtins/streams.cpp)
