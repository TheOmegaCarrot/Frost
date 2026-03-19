add_library( frost-sqlite
    sqlite.cpp
)

target_include_directories( frost-sqlite
    PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}
)

target_link_libraries( frost-sqlite
    PRIVATE
    frost-functions
    frost-extensions-common
)

add_subdirectory(tests)
