add_library( frost-symbol-table
    symbol-table.cpp
)

target_include_directories( frost-symbol-table
    PUBLIC
    include/
)

target_link_libraries( frost-symbol-table
    PUBLIC
    frost-common
    frost-value
)

macro(symbol_table_test file)
    make_test( ${file}
        LIBS
        frost-symbol-table
    )
endmacro()

symbol_table_test(tests/symbol-table.cpp)
