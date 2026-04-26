add_library( frost-ast
    array-constructor.cpp
    binop.cpp
    define.cpp
    destructure-array.cpp
    destructure-map.cpp
    do.cpp
    filter.cpp
    foreach.cpp
    format-string.cpp
    function-call.cpp
    if.cpp
    index.cpp
    literal.cpp
    map.cpp
    map-constructor.cpp
    match-alternative.cpp
    match-array.cpp
    match-binding.cpp
    match-map.cpp
    match-value.cpp
    name-lookup.cpp
    reduce.cpp
    ast-node.cpp
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
    frost-execution-context
    frost-value
)

set(AST_TEST_FILES
    basic-ast-dump
    literal
    define
    do
    if
    binop
    unop
    array-constructor
    map-constructor
    index
    name-lookup-node
    symbol-sequence
    function-call
    reduce
    map
    foreach
    filter
    format-string
    walk
    destructure-binding
    destructure-array-node
    destructure-map-node
    match
    match-alternative
    match-array
    match-binding
    match-map
    match-value
)

foreach(test_file IN LISTS AST_TEST_FILES)
    make_test( tests/${test_file}.cpp
        LIBS
        frost-ast
    )
endforeach()
