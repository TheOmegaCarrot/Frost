include(${CMAKE_CURRENT_LIST_DIR}/get-corrosion.cmake)

# cxx include path -- cxxbridge generates rust/cxx.h here.
set(FROST_CXXBRIDGE_INCLUDE
    ${CMAKE_BINARY_DIR}/cargo/build/${_CORROSION_RUST_CARGO_TARGET}/cxxbridge
)

add_subdirectory(glue)

# Register a Rust extension. Each extension directory must:
#   1. Call corrosion_import_crate() for its Cargo.toml
#   2. Create a frost-<NAME>-shim library from the generated C++
#
# This macro handles the frost-ext linkage, compile definitions, and
# ON/OFF/AUTO gating — mirroring frost_extension() for C++ extensions.
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
