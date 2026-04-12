add_executable( astviz
    astviz.cpp
)

target_link_libraries( astviz
    PRIVATE
    frost-common
    frost-parser
    Boost::json
)

set_target_properties( astviz PROPERTIES
    RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}
)
