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
)

make_test( basic-ast-dump
    tests/basic-ast-dump.cpp
)
