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
parser_test(tests/primary-expr.cpp)
parser_test(tests/program.cpp)
