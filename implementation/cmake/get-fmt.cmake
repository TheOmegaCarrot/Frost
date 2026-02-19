set(FMT_SYSTEM_HEADERS ON CACHE BOOL "Treat fmt headers as system headers" FORCE)

FetchContent_Declare( fmt
    GIT_REPOSITORY https://github.com/fmtlib/fmt
    GIT_TAG 12.1.0
)

FetchContent_MakeAvailable(fmt)
