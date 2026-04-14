find_package(ZLIB REQUIRED)

add_library( frost-compression
    compression.cpp
    deflate.cpp
    gzip.cpp
    zlib.cpp
    zlib-common.cpp
)

target_include_directories( frost-compression
    PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}
)

target_link_libraries( frost-compression
    PRIVATE
    frost-functions
    frost-extensions-common
    ZLIB::ZLIB
)

add_subdirectory(tests)
