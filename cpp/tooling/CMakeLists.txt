
option(BUILD_ASTVIZ "Build AST visualizer tool" NO)
option(BUILD_HIGHLIGHT "Build syntax highlight tool" NO)

if(BUILD_ASTVIZ OR BUILD_HIGHLIGHT)
    enable_language(C)

    CPMAddPackage(
        NAME tree-sitter
        GITHUB_REPOSITORY tree-sitter/tree-sitter
        GIT_TAG v0.26.8
    )

    add_library( frost-ts-grammar OBJECT
        ${CMAKE_SOURCE_DIR}/editor/tree-sitter-frost/src/parser.c
    )

    target_include_directories( frost-ts-grammar
        PRIVATE
        ${CMAKE_SOURCE_DIR}/editor/tree-sitter-frost/src
    )
endif()

if(BUILD_ASTVIZ)
    add_subdirectory(astviz/)
endif()

if(BUILD_HIGHLIGHT)
    add_subdirectory(highlight/)
endif()
