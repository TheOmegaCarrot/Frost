enable_language(C)

find_package(ZLIB REQUIRED)

add_library(sqlite3 STATIC sqlite3.c csv.c fileio.c sqlar.c)

target_include_directories(sqlite3
    PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}
)

target_compile_definitions(sqlite3
    PRIVATE
    SQLITE_ENABLE_FTS5
    SQLITE_ENABLE_MATH_FUNCTIONS
    SQLITE_CORE
)

target_link_libraries(sqlite3 PRIVATE ZLIB::ZLIB)
