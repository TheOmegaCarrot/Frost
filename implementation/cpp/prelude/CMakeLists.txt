add_library( frost-prelude
    prelude.cpp
)

target_include_directories( frost-prelude
    PUBLIC
    include/
)

target_link_libraries( frost-prelude
    PUBLIC
    frost-common
    frost-parser
    frost-symbol-table
)

macro(prelude_test file)
    make_test( ${file}
        LIBS
        frost-prelude
    )
endmacro()
