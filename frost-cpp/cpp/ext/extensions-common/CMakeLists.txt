add_library( frost-extensions-common INTERFACE )

target_include_directories( frost-extensions-common
    INTERFACE
    ${CMAKE_CURRENT_SOURCE_DIR}/include/
)

target_link_libraries( frost-extensions-common
    INTERFACE
    frost-functions
)
