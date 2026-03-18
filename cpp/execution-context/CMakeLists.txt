add_library( frost-execution-context
    symbol-table.cpp
)

target_include_directories( frost-execution-context
    PUBLIC
    include/
)

target_link_libraries( frost-execution-context
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
        frost-execution-context
    )
endforeach()
