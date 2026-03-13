add_library( frost-ext
    ext.cpp
)

target_include_directories( frost-ext
    PUBLIC
    include/
)

target_link_libraries( frost-ext
    PUBLIC
    frost-symbol-table
    frost-common
)

if(WITH_HTTP)
    message("Enabling HTTP")
    add_subdirectory(http)
    target_link_libraries(frost-ext PRIVATE frost-http)
    target_compile_definitions(frost-ext PRIVATE FROST_ENABLE_HTTP)
endif()
