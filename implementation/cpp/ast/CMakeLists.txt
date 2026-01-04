add_library( frost-ast
    statement.cpp
)

target_include_directories( frost-ast
    PUBLIC
    include/
)

target_link_libraries( frost-ast
    PUBLIC
    frost-common
    frost-symbol-table
    frost-value
)

make_test( tests/basic-ast-dump.cpp 
    LIBS
    frost-ast
)
