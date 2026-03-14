add_library( frost-meta
    import.cpp
    meta.cpp
)

target_include_directories( frost-meta
    PUBLIC
    include/
)

target_link_libraries( frost-meta
    PUBLIC
    frost-common
    frost-parser
    frost-functions
    frost-ast
    frost-symbol-table
    frost-prelude
    frost-ext
)

macro(meta_test file)
    make_test( ${file}
        LIBS
        frost-meta
    )
endmacro()

meta_test( tests/read-value.cpp )
