frost_ext_init()

frost_ext_find("OpenSSL" OpenSSL)

if(NOT OpenSSL_FOUND)
    frost_ext_bail("ext.http requires OpenSSL")
endif()

add_library( frost-http
    http.cpp
    request.cpp
)

target_include_directories( frost-http
    PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}
)

target_link_libraries( frost-http
    PRIVATE
    frost-functions
    frost-extensions-common
    Boost::url
    OpenSSL::SSL
    OpenSSL::Crypto
)

add_subdirectory(tests)
