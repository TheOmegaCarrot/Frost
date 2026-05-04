include(${CMAKE_CURRENT_LIST_DIR}/get-corrosion.cmake)

# cxx include path -- cxxbridge generates rust/cxx.h here.
set(FROST_CXXBRIDGE_INCLUDE
    ${CMAKE_BINARY_DIR}/cargo/build/${_CORROSION_RUST_CARGO_TARGET}/cxxbridge
)

# Collect enabled Rust extension features for the workspace root crate.
set(FROST_RUST_FEATURES "")

# Register a Rust extension. Each extension directory must:
#   1. Create a frost-<NAME>-shim library from the generated C++
#   2. Be listed as an optional dependency in rust-root/Cargo.toml
#
# This macro handles the frost-ext linkage, compile definitions, feature
# collection, and ON/OFF/AUTO gating.
macro(frost_rust_extension NAME)
    string(TOUPPER ${NAME} _UPPER)
    set(WITH_${_UPPER} "AUTO" CACHE STRING "Enable ${NAME} extension (ON/OFF/AUTO)")

    if(NOT WITH_${_UPPER} STREQUAL "OFF")
        set(FROST_EXT_MODE ${WITH_${_UPPER}})
        set(FROST_EXT_UNAVAILABLE FALSE)
        add_subdirectory(${NAME})

        if(FROST_EXT_UNAVAILABLE)
            message(STATUS "${FROST_YELLOW}ext.${NAME}: unavailable${FROST_RESET}")
        else()
            target_link_libraries(frost-ext PUBLIC frost-${NAME}-shim)
            target_compile_definitions(frost-ext PRIVATE FROST_ENABLE_${_UPPER})
            list(APPEND FROST_RUST_FEATURES ${NAME})
            message(STATUS "${FROST_GREEN}ext.${NAME}: enabled (Rust)${FROST_RESET}")
        endif()
    else()
        message(STATUS "${FROST_RED}ext.${NAME}: disabled${FROST_RESET}")
    endif()
endmacro()

# ---- Rust extensions ----
if(CMAKE_BUILD_TYPE STREQUAL "Debug")
    frost_rust_extension(example)
endif()

frost_rust_extension(csv)

# ---- Single workspace staticlib containing all enabled extensions ----
set(_RUST_FEATURES ${FROST_RUST_FEATURES})
if(BUILD_TESTS)
    list(APPEND _RUST_FEATURES test-helpers)
endif()

corrosion_import_crate(
    MANIFEST_PATH ${CMAKE_CURRENT_SOURCE_DIR}/rust-root/Cargo.toml
    FEATURES ${_RUST_FEATURES}
)

# Path to the single workspace staticlib (Corrosion copies it here).
set(FROST_RUST_STATICLIB
    ${CMAKE_CURRENT_BINARY_DIR}/libfrost_rust_ext.a
)

add_subdirectory(glue)
