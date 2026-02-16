add_library( frost-ast
    array-constructor.cpp
    array-destructure.cpp
    binop.cpp
    define.cpp
    filter.cpp
    foreach.cpp
    format-string.cpp
    function_call.cpp
    if.cpp
    index.cpp
    literal.cpp
    map.cpp
    map-constructor.cpp
    map-destructure.cpp
    name_lookup.cpp
    reduce.cpp
    statement.cpp
    unop.cpp
)

target_include_directories( frost-ast
    PUBLIC
    include/
)

target_link_libraries( frost-ast
    PUBLIC
    frost-common
    frost-utils
    frost-symbol-table
    frost-value
)

set(AST_TEST_FILES
    basic-ast-dump
    literal
    define
    array-destructure
    map-destructure
    if
    binop
    unop
    array-constructor
    map-constructor
    index
    name_lookup
    symbol_sequence
    function_call
    reduce
    map
    foreach
    filter
    format-string
)

foreach(test_file IN LISTS AST_TEST_FILES)
    make_test( tests/${test_file}.cpp
        LIBS
        frost-ast
    )
endforeach()
