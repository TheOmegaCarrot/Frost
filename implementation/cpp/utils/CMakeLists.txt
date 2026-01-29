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

macro(utils_test file)
    make_test( ${file}
        LIBS
        frost-utils
    )
endmacro()

utils_test(tests/parse-fmt-string.cpp)
