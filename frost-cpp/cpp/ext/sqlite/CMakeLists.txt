add_subdirectory(vendored-sqlite)

add_library( frost-sqlite
    connection.cpp
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
    sqlite3
)

if(SQLITE_HAVE_ZLIB)
    target_compile_definitions(frost-sqlite PRIVATE FROST_SQLITE_HAVE_ZLIB)
endif()

add_subdirectory(tests)
