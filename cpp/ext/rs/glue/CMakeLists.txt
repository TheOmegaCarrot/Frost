corrosion_import_crate(MANIFEST_PATH ${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml)

# C++ implementations of the glue functions -- compiled by CMake so it
# has access to the full frost include tree.
add_library(frost-rs-glue
    cpp/frost-glue.cpp
)

add_dependencies(frost-rs-glue cargo-build_frost_glue)

target_include_directories(frost-rs-glue
    PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/cpp
    ${FROST_CXXBRIDGE_INCLUDE}
)

# Rust symbols (cxx bridge code) come through extension staticlibs, which
# include frost-glue as an rlib dependency. Don't link libfrost_glue.a
# separately -- it would duplicate symbols with every extension.
# Multiple extension staticlibs contain identical copies of glue symbols;
# --allow-multiple-definition is safe here (same source, same compilation).
# The proper fix is a Cargo workspace (single staticlib), tracked for later.
target_link_libraries(frost-rs-glue
    PUBLIC
    frost-value
    frost-functions
)
target_link_options(frost-rs-glue
    PUBLIC
    "LINKER:--allow-multiple-definition"
)

if(BUILD_TESTS)
    corrosion_import_crate(
        MANIFEST_PATH ${CMAKE_CURRENT_SOURCE_DIR}/tests/rust-helpers/Cargo.toml
    )

    make_test(tests/glue.cpp
        LIBS
        frost-rs-glue
        ${CMAKE_CURRENT_BINARY_DIR}/libfrost_glue_test_helpers.a
    )
endif()
