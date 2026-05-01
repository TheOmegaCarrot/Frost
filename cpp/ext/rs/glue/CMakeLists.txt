corrosion_import_crate(MANIFEST_PATH ${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml)

# C++ implementations of the glue functions -- compiled by CMake so it
# has access to the full frost include tree.
add_library(frost-rs-glue
    cpp/frost-glue.cpp
)

target_include_directories(frost-rs-glue
    PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/cpp
)

target_link_libraries(frost-rs-glue
    PUBLIC
    frost-value
    frost-glue
)
