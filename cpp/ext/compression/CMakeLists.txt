find_package(ZLIB REQUIRED)

set(BROTLI_BUILD_TOOLS OFF CACHE BOOL "" FORCE)
CPMAddPackage(
    NAME brotli
    GITHUB_REPOSITORY google/brotli
    GIT_TAG v1.2.0
)

set(ZSTD_BUILD_PROGRAMS OFF CACHE BOOL "" FORCE)
set(ZSTD_BUILD_TESTS OFF CACHE BOOL "" FORCE)
set(ZSTD_BUILD_CONTRIB OFF CACHE BOOL "" FORCE)
CPMAddPackage(
    NAME zstd
    GITHUB_REPOSITORY facebook/zstd
    GIT_TAG v1.5.7
    SOURCE_SUBDIR build/cmake
)

add_library( frost-compression
    compression.cpp
    brotli.cpp
    deflate.cpp
    gzip.cpp
    zstd.cpp
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
    libzstd_static
)

add_subdirectory(tests)
