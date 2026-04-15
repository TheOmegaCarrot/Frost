add_library( frost-ext
    ext.cpp
)

target_include_directories( frost-ext
    PUBLIC
    include/
)

target_link_libraries( frost-ext
    PUBLIC
    frost-functions
    frost-common
)

add_subdirectory(extensions-common/)

# ANSI color codes for status messages
string(ASCII 27 _ESC)
set(FROST_GREEN "${_ESC}[32m")
set(FROST_RED "${_ESC}[31m")
set(FROST_YELLOW "${_ESC}[33m")
set(FROST_RESET "${_ESC}[0m")

# Register a Frost extension.
# WITH_<UPPER_NAME> is a three-way setting: ON, OFF, or AUTO (default).
#   ON   -- enable, hard-error if dependencies are missing
#   AUTO -- enable if dependencies are found, skip otherwise
#   OFF  -- skip entirely
# The extension's CMakeLists.txt receives FROST_EXT_MODE set to ON or AUTO.
# If an extension cannot satisfy its dependencies under AUTO, it must set
# FROST_EXT_UNAVAILABLE to TRUE and return() early.
macro(frost_extension NAME)
    string(TOUPPER ${NAME} _UPPER)
    set(WITH_${_UPPER} "AUTO" CACHE STRING "Enable ${NAME} extension (ON/OFF/AUTO)")

    if(NOT WITH_${_UPPER} STREQUAL "OFF")
        set(FROST_EXT_MODE ${WITH_${_UPPER}})
        set(FROST_EXT_UNAVAILABLE FALSE)
        add_subdirectory(${NAME})

        if(FROST_EXT_UNAVAILABLE)
            message(STATUS "${FROST_YELLOW}ext.${NAME}: unavailable${FROST_RESET}")
        else()
            message(STATUS "${FROST_GREEN}ext.${NAME}: enabled${FROST_RESET}")
            target_link_libraries(frost-ext PRIVATE frost-${NAME})
            target_compile_definitions(frost-ext PRIVATE FROST_ENABLE_${_UPPER})
        endif()
    else()
        message(STATUS "${FROST_RED}ext.${NAME}: disabled${FROST_RESET}")
    endif()
endmacro()

frost_extension(hash)
frost_extension(http)
frost_extension(sqlite)
frost_extension(compression)
frost_extension(unsafe)

