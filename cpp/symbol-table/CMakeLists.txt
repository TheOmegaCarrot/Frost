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

set(SYMBOL_TABLE_TEST_FILES
    symbol-table
)

foreach(test_file IN LISTS SYMBOL_TABLE_TEST_FILES)
    make_test( tests/${test_file}.cpp
        LIBS
        frost-symbol-table
    )
endforeach()
