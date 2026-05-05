include(${CMAKE_CURRENT_LIST_DIR}/get-corrosion.cmake)

# cxx include path -- cxxbridge generates rust/cxx.h here.
# Reconstruct Corrosion's internal cargo target directory (hash-bucketed since 0.6).
set(_ws_manifest ${CMAKE_CURRENT_SOURCE_DIR}/rust-root/Cargo.toml)
cmake_path(GET _ws_manifest PARENT_PATH _parent)
cmake_path(GET _parent PARENT_PATH _grandparent)
string(REPLACE "${_grandparent}/" "" _cargo_folder "${_parent}")
string(SHA1 _cargo_hash "${_ws_manifest}")
string(SUBSTRING "${_cargo_hash}" 0 5 _cargo_hash)

set(FROST_CXXBRIDGE_INCLUDE
    ${CMAKE_BINARY_DIR}/cargo/${_cargo_folder}_${_cargo_hash}/${_CORROSION_RUST_CARGO_TARGET}/cxxbridge
)

# Collect enabled Rust extension features for the workspace root crate.
set(FROST_RUST_FEATURES "")

# Register a Rust extension. Each extension directory must:
#   1. Create a frost-<NAME>-shim library from the generated C++
#   2. Be listed as an optional dependency in rust-root/Cargo.toml
#
# This macro handles the frost-ext linkage, feature collection, and
# ON/OFF/AUTO gating.
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

add_subdirectory(glue)

list(APPEND FROST_ENABLED_EXTENSIONS ${FROST_RUST_FEATURES})
set(FROST_ENABLED_EXTENSIONS "${FROST_ENABLED_EXTENSIONS}" PARENT_SCOPE)
