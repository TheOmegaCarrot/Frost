add_library( frost-ast
    index.cpp
    map.cpp
    reduce.cpp
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

make_ast_test(tests/binop.cpp)

make_ast_test(tests/unop.cpp)

make_ast_test(tests/array-constructor.cpp)

make_ast_test(tests/map-constructor.cpp)

make_ast_test(tests/index.cpp)

make_ast_test(tests/symbol_sequence.cpp)

make_ast_test(tests/function_call.cpp)

make_ast_test(tests/reduce.cpp)

make_ast_test(tests/map.cpp)
