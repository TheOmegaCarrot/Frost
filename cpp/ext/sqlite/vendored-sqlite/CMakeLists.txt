enable_language(C)

add_library(sqlite3 STATIC sqlite3.c csv.c decimal.c fileio.c regexp.c series.c shathree.c sqlar.c uuid.c zipfile.c)

target_include_directories(sqlite3
    PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}
)

target_compile_definitions(sqlite3
    PRIVATE
    SQLITE_CORE

    # Features
    SQLITE_ENABLE_DBSTAT_VTAB
    SQLITE_ENABLE_FTS5
    SQLITE_ENABLE_MATH_FUNCTIONS
    SQLITE_ENABLE_PERCENTILE
    SQLITE_ENABLE_STAT4
    SQLITE_SOUNDEX

    # Defaults
    SQLITE_DEFAULT_FOREIGN_KEYS=1
    SQLITE_DEFAULT_MEMSTATUS=0
    SQLITE_TEMP_STORE=2

    # Performance
    SQLITE_LIKE_DOESNT_MATCH_BLOBS
    SQLITE_MAX_EXPR_DEPTH=0
    SQLITE_USE_ALLOCA

    # Cleanup
    SQLITE_OMIT_UTF16
)

find_package(ZLIB REQUIRED)
target_link_libraries(sqlite3 PRIVATE ZLIB::ZLIB)
