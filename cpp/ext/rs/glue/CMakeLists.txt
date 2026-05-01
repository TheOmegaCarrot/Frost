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

# Corrosion copies libfrost_glue.a here after cargo builds it.
# Link against the file path directly since the Corrosion IMPORTED target
# doesn't set IMPORTED_LOCATION correctly for mold.
target_link_libraries(frost-rs-glue
    PUBLIC
    frost-value
    ${CMAKE_CURRENT_BINARY_DIR}/libfrost_glue.a
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
