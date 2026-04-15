frost_ext_init()

frost_ext_find("OpenSSL" OpenSSL)
frost_ext_find("zlib" ZLIB)

if(NOT OpenSSL_FOUND OR NOT ZLIB_FOUND)
    frost_ext_bail("ext.hash requires OpenSSL and zlib")
endif()

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

add_subdirectory(tests)
