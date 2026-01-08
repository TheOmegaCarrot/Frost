add_library( frost-functions
    closure.cpp
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

macro(closure_test file)
    make_test( ${file}
        LIBS
        frost-functions
    )
endmacro()

