frost_ext_find("zlib (deflate, gzip, zlib)" ZLIB)
frost_ext_find("bz2" BZip2)
frost_ext_pkg("brotli" BROTLI libbrotlienc libbrotlidec)
frost_ext_find("xz" LibLZMA)
frost_ext_pkg("lz4" LZ4 liblz4)
frost_ext_pkg("zstd" ZSTD libzstd)

if(NOT ZLIB_FOUND AND NOT BZip2_FOUND AND NOT LibLZMA_FOUND AND NOT BROTLI_FOUND AND NOT LZ4_FOUND AND NOT ZSTD_FOUND)
    if(FROST_EXT_MODE STREQUAL "ON")
        message(FATAL_ERROR
            "ext.compression enabled but no compression libraries found.\n"
            "Install at least one of: zlib, bz2, brotli, lz4, zstd")
    endif()
    set(FROST_EXT_UNAVAILABLE TRUE PARENT_SCOPE)
    return()
endif()

set(COMPRESSION_SOURCES compression.cpp)
set(COMPRESSION_LIBS frost-functions frost-extensions-common)
set(COMPRESSION_DEFS "")

if(ZLIB_FOUND)
    list(APPEND COMPRESSION_SOURCES deflate.cpp gzip.cpp zlib.cpp zlib-common.cpp)
    list(APPEND COMPRESSION_LIBS ZLIB::ZLIB)
    list(APPEND COMPRESSION_DEFS FROST_HAVE_ZLIB)
endif()

if(BZip2_FOUND)
    list(APPEND COMPRESSION_SOURCES bz2.cpp)
    list(APPEND COMPRESSION_LIBS BZip2::BZip2)
    list(APPEND COMPRESSION_DEFS FROST_HAVE_BZ2)
endif()

if(LibLZMA_FOUND)
    list(APPEND COMPRESSION_SOURCES xz.cpp)
    list(APPEND COMPRESSION_LIBS LibLZMA::LibLZMA)
    list(APPEND COMPRESSION_DEFS FROST_HAVE_XZ)
endif()

if(LZ4_FOUND)
    list(APPEND COMPRESSION_SOURCES lz4.cpp)
    list(APPEND COMPRESSION_LIBS PkgConfig::LZ4)
    list(APPEND COMPRESSION_DEFS FROST_HAVE_LZ4)
endif()

if(BROTLI_FOUND)
    list(APPEND COMPRESSION_SOURCES brotli.cpp)
    list(APPEND COMPRESSION_LIBS PkgConfig::BROTLI)
    list(APPEND COMPRESSION_DEFS FROST_HAVE_BROTLI)
endif()

if(ZSTD_FOUND)
    list(APPEND COMPRESSION_SOURCES zstd.cpp)
    list(APPEND COMPRESSION_LIBS PkgConfig::ZSTD)
    list(APPEND COMPRESSION_DEFS FROST_HAVE_ZSTD)
endif()

add_library( frost-compression
    ${COMPRESSION_SOURCES}
)

target_include_directories( frost-compression
    PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}
)

target_link_libraries( frost-compression
    PRIVATE
    ${COMPRESSION_LIBS}
)

target_compile_definitions( frost-compression
    PRIVATE
    ${COMPRESSION_DEFS}
)

add_subdirectory(tests)
