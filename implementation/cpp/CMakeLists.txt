add_subdirectory(utils/)
add_subdirectory(value/)
add_subdirectory(symbol-table/)
add_subdirectory(ast/)
add_subdirectory(functions/)
add_subdirectory(parser/)
add_subdirectory(prelude/)
add_subdirectory(import/)

add_executable( frost
    frost.cpp
    repl.cpp
)

target_link_libraries( frost
    PUBLIC
    frost-parser
    frost-ast
    frost-functions
    frost-symbol-table
    frost-value
    frost-prelude
    frost-import
    frost-common
    replxx::replxx
)

set_target_properties( frost PROPERTIES
    RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}
)
