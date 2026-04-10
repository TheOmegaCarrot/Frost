find_package(OpenSSL REQUIRED)
find_package(ZLIB REQUIRED)

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
