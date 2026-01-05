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

macro(make_ast_test file)
    make_test( ${file}
        LIBS
        frost-ast
    )
endmacro()

make_ast_test(tests/basic-ast-dump.cpp)

make_ast_test(tests/literal.cpp)

make_ast_test(tests/define.cpp)

make_ast_test(tests/if.cpp)
