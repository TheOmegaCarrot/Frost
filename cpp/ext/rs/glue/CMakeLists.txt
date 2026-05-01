corrosion_import_crate(MANIFEST_PATH ${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml)

# C++ implementations of the glue functions -- compiled by CMake so it
# has access to the full frost include tree.
add_library(frost-rs-glue
    cpp/frost-glue.cpp
)

target_include_directories(frost-rs-glue
    PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/cpp
    ${FROST_CXXBRIDGE_INCLUDE}
)

# Rust symbols (cxx bridge code) come through extension staticlibs, which
# include frost-glue as an rlib dependency. Don't link libfrost_glue.a
# separately -- it would duplicate symbols with every extension.
target_link_libraries(frost-rs-glue
    PUBLIC
    frost-value
    frost-functions
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
