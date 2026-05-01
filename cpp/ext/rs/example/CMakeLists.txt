corrosion_import_crate(MANIFEST_PATH ${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml)

set(_GEN_CPP ${CMAKE_CURRENT_SOURCE_DIR}/generated/example.cpp)
set_source_files_properties(${_GEN_CPP} PROPERTIES GENERATED TRUE)

add_library(frost-example-shim
    ${_GEN_CPP}
)

add_dependencies(frost-example-shim cargo-build_frost_example)

target_include_directories(frost-example-shim
    PRIVATE
    ${CMAKE_BINARY_DIR}/cargo/build/x86_64-unknown-linux-gnu/cxxbridge
)

target_link_libraries(frost-example-shim
    PRIVATE
    frost-functions
    frost-extensions-common
    PUBLIC
    frost-rs-glue
    ${CMAKE_CURRENT_BINARY_DIR}/libfrost_example.a
)
