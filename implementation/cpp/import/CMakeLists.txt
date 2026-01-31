add_library( frost-import
    import.cpp
)

target_include_directories( frost-import
    PUBLIC
    include/
)

target_link_libraries( frost-import
    PUBLIC
    frost-common
    frost-parser
    frost-functions
    frost-ast
    frost-symbol-table
    frost-prelude
)

macro(import_test file)
    make_test( ${file}
        LIBS
        frost-import
    )
endmacro()

