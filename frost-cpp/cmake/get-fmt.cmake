set(FMT_SYSTEM_HEADERS ON CACHE BOOL "Treat fmt headers as system headers" FORCE)

CPMAddPackage(
    NAME fmt
    GITHUB_REPOSITORY fmtlib/fmt
    GIT_TAG 12.1.0
)
