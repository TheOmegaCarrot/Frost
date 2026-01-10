add_library( frost-functions
    closure.cpp
    builtins.cpp
    builtins/map-ops.cpp
    builtins/type-tests.cpp
    builtins/type-conversions.cpp
    builtins/pack_call.cpp
    builtins/debug-helpers.cpp
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

functions_test(tests/builtins/pack_call.cpp)
