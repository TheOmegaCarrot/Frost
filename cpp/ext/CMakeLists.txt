add_library( frost-ext
    ext.cpp
)

target_include_directories( frost-ext
    PUBLIC
    include/
)

target_link_libraries( frost-ext
    PUBLIC
    frost-execution-context
    frost-common
)

add_subdirectory(extensions-common/)

option(WITH_HTTP "Enable HTTP support" YES)
option(WITH_SQLITE "Enable SQLite support" YES)

if(WITH_HTTP)
    message("Enabling HTTP")
    add_subdirectory(http)
    target_link_libraries(frost-ext PRIVATE frost-http)
    target_compile_definitions(frost-ext PRIVATE FROST_ENABLE_HTTP)
endif()

if(WITH_SQLITE)
    message("Enabling SQLite")
    add_subdirectory(sqlite)
    target_link_libraries(frost-ext PRIVATE frost-sqlite)
    target_compile_definitions(frost-ext PRIVATE FROST_ENABLE_SQLITE)
endif()
