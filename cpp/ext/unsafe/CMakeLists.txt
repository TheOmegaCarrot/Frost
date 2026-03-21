add_library( frost-unsafe
    unsafe.cpp
)

target_include_directories( frost-unsafe
    PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}
)

target_link_libraries( frost-unsafe
    PRIVATE
    frost-functions
    frost-extensions-common
)

make_test( tests/unsafe.cpp
    LIBS
    frost-unsafe
    frost-extensions-common
)
