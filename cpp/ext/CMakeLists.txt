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

find_package(PkgConfig REQUIRED)

# ---- Helpers for extension CMakeLists.txt files ----
#
# Extensions receive two variables from frost_extension:
#   FROST_EXT_MODE  -- "ON" or "AUTO"
#   _REQ            -- "REQUIRED" or "QUIET" (derived from FROST_EXT_MODE)
#
# frost_ext_find and frost_ext_pkg wrap the standard CMake dependency finders
# with colored status output and automatic REQUIRED/QUIET handling.
#
# If an extension cannot satisfy its dependencies under AUTO, it should set
# FROST_EXT_UNAVAILABLE to TRUE (in PARENT_SCOPE) and return().

# Wraps find_package with a colored status message.
#   frost_ext_find(LABEL PACKAGE)
# Sets <PACKAGE>_FOUND.
macro(frost_ext_find LABEL PACKAGE)
    find_package(${PACKAGE} ${_REQ})
    if(${PACKAGE}_FOUND)
        message(STATUS "  ${FROST_GREEN}${LABEL}: found${FROST_RESET}")
    else()
        message(STATUS "  ${FROST_RED}${LABEL}: not found${FROST_RESET}")
    endif()
endmacro()

# Wraps pkg_check_modules with a colored status message.
#   frost_ext_pkg(LABEL VAR_NAME pkg_names...)
# Sets <VAR_NAME>_FOUND and creates PkgConfig::<VAR_NAME>.
macro(frost_ext_pkg LABEL VAR_NAME)
    pkg_check_modules(${VAR_NAME} ${_REQ} IMPORTED_TARGET ${ARGN})
    if(${VAR_NAME}_FOUND)
        message(STATUS "  ${FROST_GREEN}${LABEL}: found${FROST_RESET}")
    else()
        message(STATUS "  ${FROST_RED}${LABEL}: not found${FROST_RESET}")
    endif()
endmacro()

# Register a Frost extension.
# WITH_<UPPER_NAME> is a three-way setting: ON, OFF, or AUTO (default).
#   ON   -- enable, hard-error if dependencies are missing
#   AUTO -- enable if dependencies are found, skip otherwise
#   OFF  -- skip entirely
macro(frost_extension NAME)
    string(TOUPPER ${NAME} _UPPER)
    set(WITH_${_UPPER} "AUTO" CACHE STRING "Enable ${NAME} extension (ON/OFF/AUTO)")

    if(NOT WITH_${_UPPER} STREQUAL "OFF")
        set(FROST_EXT_MODE ${WITH_${_UPPER}})
        if(FROST_EXT_MODE STREQUAL "ON")
            set(_REQ REQUIRED)
        else()
            set(_REQ QUIET)
        endif()
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
frost_extension(toml)
frost_extension(unsafe)

