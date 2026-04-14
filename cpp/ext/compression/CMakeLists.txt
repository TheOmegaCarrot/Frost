find_package(ZLIB REQUIRED)

set(BROTLI_BUILD_TOOLS OFF CACHE BOOL "" FORCE)
CPMAddPackage(
    NAME brotli
    GITHUB_REPOSITORY google/brotli
    GIT_TAG v1.2.0
)

add_library( frost-compression
    compression.cpp
    brotli.cpp
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
    brotlienc
    brotlidec
)

add_subdirectory(tests)
