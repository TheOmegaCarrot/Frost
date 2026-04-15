frost_ext_find("OpenSSL" OpenSSL)

if(NOT OpenSSL_FOUND)
    if(FROST_EXT_MODE STREQUAL "ON")
        message(FATAL_ERROR "ext.http requires OpenSSL")
    endif()
    set(FROST_EXT_UNAVAILABLE TRUE PARENT_SCOPE)
    return()
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
