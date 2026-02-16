add_library( frost-parser
    parser.cpp
)

target_include_directories( frost-parser
    PUBLIC
    include/
)

target_link_libraries( frost-parser
    PUBLIC
    frost-common
    frost-ast
    frost-functions
    foonathan::lexy
)

set(PARSER_TEST_FILES
    name-lookup
    identifiers
    literals-numbers
    literals-strings
    literals-keywords
    map-expr
    filter-expr
    reduce-expr
    foreach-expr
    array-literals
    map-literals
    format-string-expr
    primary-expr
    if-expr
    define-stmt
    program
    program-stress
    postfix-expr
    unary-expr
    binary-expr
    lambda-expr
    threaded-call-expr
)

foreach(test_file IN LISTS PARSER_TEST_FILES)
    make_test( tests/${test_file}.cpp
        LIBS
        frost-parser
    )
endforeach()
