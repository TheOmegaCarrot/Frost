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
)

macro(parser_test file)
    make_test( ${file}
        LIBS
        frost-parser
    )
endmacro()

