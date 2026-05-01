include(${CMAKE_CURRENT_LIST_DIR}/get-corrosion.cmake)

add_subdirectory(glue)

# Like frost_extension(), but for Rust extensions where Corrosion owns the
# crate target name (frost-<NAME>). The C++ shim target is frost-<NAME>-shim.
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
frost_rust_extension(example)
