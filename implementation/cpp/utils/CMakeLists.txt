add_library( frost-utils
    parse-fmt-string.cpp
)

target_include_directories( frost-utils
    PUBLIC
    include/
)

target_link_libraries( frost-utils
    PUBLIC
    frost-common
)

set(UTILS_TEST_FILES
    parse-fmt-string
)

foreach(test_file IN LISTS UTILS_TEST_FILES)
    make_test( tests/${test_file}.cpp
        LIBS
        frost-utils
    )
endforeach()
