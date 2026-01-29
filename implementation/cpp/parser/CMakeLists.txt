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

macro(parser_test file)
    make_test( ${file}
        LIBS
        frost-parser
    )
endmacro()

parser_test(tests/name-lookup.cpp)
parser_test(tests/identifiers.cpp)
parser_test(tests/literals-numbers.cpp)
parser_test(tests/literals-strings.cpp)
parser_test(tests/literals-keywords.cpp)
parser_test(tests/map-expr.cpp)
parser_test(tests/filter-expr.cpp)
parser_test(tests/reduce-expr.cpp)
parser_test(tests/foreach-expr.cpp)
parser_test(tests/array-literals.cpp)
parser_test(tests/map-literals.cpp)
parser_test(tests/format-string-expr.cpp)
parser_test(tests/primary-expr.cpp)
parser_test(tests/if-expr.cpp)
parser_test(tests/define-stmt.cpp)
parser_test(tests/program.cpp)
parser_test(tests/program-stress.cpp)
parser_test(tests/postfix-expr.cpp)
parser_test(tests/unary-expr.cpp)
parser_test(tests/binary-expr.cpp)
parser_test(tests/lambda-expr.cpp)
parser_test(tests/ufcs-expr.cpp)
