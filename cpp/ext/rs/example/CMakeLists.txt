corrosion_import_crate(MANIFEST_PATH ${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml)

# build.rs writes example.cpp into cargo's OUT_DIR. Since the path includes
# a hash we can't predict at configure time, copy it to a known location
# after the cargo build completes.
set(_GEN_CPP ${CMAKE_CURRENT_BINARY_DIR}/example.cpp)

add_custom_target(frost-example-gen
    COMMAND ${CMAKE_COMMAND}
        -DCARGO_BUILD_DIR=${CMAKE_BINARY_DIR}/cargo/build/x86_64-unknown-linux-gnu/debug
        -DDEST=${_GEN_CPP}
        -P ${CMAKE_CURRENT_SOURCE_DIR}/copy-generated.cmake
    DEPENDS cargo-build_frost_example
    COMMENT "Copying generated example.cpp from cargo OUT_DIR"
    BYPRODUCTS ${_GEN_CPP}
)

add_library(frost-example-shim
    ${_GEN_CPP}
)

set_source_files_properties(${_GEN_CPP} PROPERTIES GENERATED TRUE)

add_dependencies(frost-example-shim frost-example-gen)

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
