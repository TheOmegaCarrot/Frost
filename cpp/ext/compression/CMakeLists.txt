find_package(PkgConfig REQUIRED)

# Dependency detection -- FROST_EXT_MODE is ON or AUTO
if(FROST_EXT_MODE STREQUAL "ON")
    set(_REQ REQUIRED)
else()
    set(_REQ QUIET)
endif()

find_package(ZLIB ${_REQ})
pkg_check_modules(BROTLI ${_REQ} IMPORTED_TARGET libbrotlienc libbrotlidec)
pkg_check_modules(ZSTD ${_REQ} IMPORTED_TARGET libzstd)

# Require at least one algorithm, or bail under AUTO
if(NOT ZLIB_FOUND AND NOT BROTLI_FOUND AND NOT ZSTD_FOUND)
    if(FROST_EXT_MODE STREQUAL "ON")
        message(FATAL_ERROR
            "ext.compression enabled but no compression libraries found.\n"
            "Install at least one of: zlib, brotli, zstd (development headers)")
    else()
        set(FROST_EXT_UNAVAILABLE TRUE PARENT_SCOPE)
        return()
    endif()
endif()

set(COMPRESSION_SOURCES compression.cpp)
set(COMPRESSION_LIBS frost-functions frost-extensions-common)
set(COMPRESSION_DEFS "")

if(ZLIB_FOUND)
    message(STATUS "  ${FROST_GREEN}zlib (deflate, gzip, zlib): found${FROST_RESET}")
    list(APPEND COMPRESSION_SOURCES deflate.cpp gzip.cpp zlib.cpp zlib-common.cpp)
    list(APPEND COMPRESSION_LIBS ZLIB::ZLIB)
    list(APPEND COMPRESSION_DEFS FROST_HAVE_ZLIB)
else()
    message(STATUS "  ${FROST_RED}zlib (deflate, gzip, zlib): not found${FROST_RESET}")
endif()

if(BROTLI_FOUND)
    message(STATUS "  ${FROST_GREEN}brotli: found${FROST_RESET}")
    list(APPEND COMPRESSION_SOURCES brotli.cpp)
    list(APPEND COMPRESSION_LIBS PkgConfig::BROTLI)
    list(APPEND COMPRESSION_DEFS FROST_HAVE_BROTLI)
else()
    message(STATUS "  ${FROST_RED}brotli: not found${FROST_RESET}")
endif()

if(ZSTD_FOUND)
    message(STATUS "  ${FROST_GREEN}zstd: found${FROST_RESET}")
    list(APPEND COMPRESSION_SOURCES zstd.cpp)
    list(APPEND COMPRESSION_LIBS PkgConfig::ZSTD)
    list(APPEND COMPRESSION_DEFS FROST_HAVE_ZSTD)
else()
    message(STATUS "  ${FROST_RED}zstd: not found${FROST_RESET}")
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
