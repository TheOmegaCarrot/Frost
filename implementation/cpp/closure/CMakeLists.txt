add_library( frost-closure
    closure.cpp
)

target_include_directories( frost-closure
    PUBLIC
    include/
)

target_link_libraries( frost-closure
    PUBLIC
    frost-common
    frost-ast
    frost-value
    frost-symbol-table
)

macro(closure_test file)
    make_test( ${file}
        LIBS
        frost-closure
    )
endmacro()

