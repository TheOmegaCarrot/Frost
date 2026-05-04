# C++ implementations of the glue functions -- compiled by CMake so it
# has access to the full frost include tree.
add_library(frost-rs-glue
    cpp/frost-glue.cpp
)

add_dependencies(frost-rs-glue cargo-build_frost_rust_ext)

target_include_directories(frost-rs-glue
    PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/cpp
    ${FROST_CXXBRIDGE_INCLUDE}
)

# All Rust symbols come from the single workspace staticlib.
target_link_libraries(frost-rs-glue
    PUBLIC
    frost-value
    frost-functions
    ${FROST_RUST_STATICLIB}
)

if(BUILD_TESTS)
    make_test(tests/glue.cpp
        LIBS
        frost-rs-glue
    )
endif()
