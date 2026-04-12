enable_language(C)

CPMAddPackage(
    NAME tree-sitter
    GITHUB_REPOSITORY tree-sitter/tree-sitter
    GIT_TAG v0.26.6
)

# Frost tree-sitter grammar (generated parser.c)
add_library( frost-ts-grammar OBJECT
    ${CMAKE_SOURCE_DIR}/editor/tree-sitter-frost/src/parser.c
)

target_include_directories( frost-ts-grammar
    PRIVATE
    ${CMAKE_SOURCE_DIR}/editor/tree-sitter-frost/src
)

add_executable( astviz
    astviz.cpp
)

target_link_libraries( astviz
    PRIVATE
    frost-common
    frost-parser
    frost-ts-grammar
    tree-sitter
    Boost::json
)

# --embed-dir for #embed "highlights.scm" (GCC uses separate search path for #embed)
target_compile_options( astviz
    PRIVATE
    --embed-dir=${CMAKE_SOURCE_DIR}/editor/tree-sitter-frost/queries
)

set_target_properties( astviz PROPERTIES
    RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}
)
