CPMAddPackage(
    NAME SQLiteCpp
    GITHUB_REPOSITORY SRombauts/SQLiteCpp
    GIT_TAG 3.3.3
    OPTIONS
        "SQLITECPP_RUN_CPPLINT OFF"
        "SQLITECPP_RUN_CPPCHECK OFF"
)

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
    SQLiteCpp
)

add_subdirectory(tests)
