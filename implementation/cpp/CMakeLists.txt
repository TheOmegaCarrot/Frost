add_subdirectory(value/)
add_subdirectory(symbol-table/)
add_subdirectory(ast/)
add_subdirectory(functions/)
add_subdirectory(parser/)

add_executable( frost
    frost.cpp
)

target_link_libraries( frost
    PRIVATE
    frost-parser
    frost-ast
    frost-functions
    frost-symbol-table
    frost-value
)
