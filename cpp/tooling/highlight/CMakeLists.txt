add_executable( frost-highlight
    highlight.cpp
)

target_link_libraries( frost-highlight
    PRIVATE
    frost-common
    frost-ts-grammar
    tree-sitter
)

target_compile_options( frost-highlight
    PRIVATE
    --embed-dir=${CMAKE_SOURCE_DIR}/editor/tree-sitter-frost/queries
)

set_target_properties( frost-highlight PROPERTIES
    RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}
)
