add_library( frost-functions
    closure.cpp
    builtins.cpp
    builtins/debug-helpers.cpp
    builtins/format.cpp
    builtins/pack_call.cpp
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
    frost-ast
    frost-value
    frost-symbol-table
)

macro(functions_test file)
    make_test( ${file}
        LIBS
        frost-functions
    )
endmacro()

functions_test(tests/builtin.cpp)

functions_test(tests/builtins/keys.cpp)

functions_test(tests/builtins/type-tests.cpp)

functions_test(tests/builtins/type-conversions.cpp)

functions_test(tests/builtins/values.cpp)

functions_test(tests/builtins/len.cpp)

functions_test(tests/builtins/pack_call.cpp)

functions_test(tests/builtins/debug-helpers.cpp)

functions_test(tests/builtins/pformat.cpp)

functions_test(tests/closure.cpp)

functions_test(tests/lambda.cpp)
