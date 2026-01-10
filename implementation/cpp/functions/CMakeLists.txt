add_library( frost-functions
    closure.cpp
    builtins.cpp
    builtins/map-ops.cpp
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

functions_test(tests/builtins/keys.cpp)

functions_test(tests/builtins/values.cpp)
