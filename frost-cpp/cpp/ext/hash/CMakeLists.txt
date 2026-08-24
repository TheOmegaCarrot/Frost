frost_ext_find("OpenSSL" OpenSSL)
frost_ext_find("zlib" ZLIB)

if(NOT OpenSSL_FOUND OR NOT ZLIB_FOUND)
    if(FROST_EXT_MODE STREQUAL "ON")
        message(FATAL_ERROR "ext.hash requires OpenSSL and zlib")
    endif()
    set(FROST_EXT_UNAVAILABLE TRUE PARENT_SCOPE)
    return()
endif()

CPMAddPackage(
    NAME xxHash
    GITHUB_REPOSITORY Cyan4973/xxHash
    GIT_TAG v0.8.3
    DOWNLOAD_ONLY YES
)

add_library( frost-hash
    hash.cpp
)

target_include_directories( frost-hash
    PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}
)

target_link_libraries( frost-hash
    PRIVATE
    frost-functions
    frost-extensions-common
    OpenSSL::Crypto
    ZLIB::ZLIB
)

target_include_directories( frost-hash
    PRIVATE
    ${xxHash_SOURCE_DIR}
)

add_subdirectory(tests)
