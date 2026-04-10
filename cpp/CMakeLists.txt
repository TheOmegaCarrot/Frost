add_subdirectory(utils/)
add_subdirectory(value/)
add_subdirectory(execution-context/)
add_subdirectory(ast/)
add_subdirectory(functions/)
add_subdirectory(parser/)
add_subdirectory(prelude/)
add_subdirectory(meta/)

add_executable( frost
    frost.cpp
    repl.cpp
)

add_subdirectory(ext/)

target_link_libraries( frost
    PUBLIC
    frost-parser
    frost-ast
    frost-functions
    frost-execution-context
    frost-value
    frost-prelude
    frost-meta
    frost-ext
    frost-common
    replxx::replxx
)

set_target_properties( frost PROPERTIES
    RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}
)
