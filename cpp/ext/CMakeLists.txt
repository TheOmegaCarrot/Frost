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

# Call at the top of an extension's CMakeLists.txt.
# Sets _REQ to REQUIRED or QUIET based on FROST_EXT_MODE.
macro(frost_ext_init)
    if(FROST_EXT_MODE STREQUAL "ON")
        set(_REQ REQUIRED)
    else()
        set(_REQ QUIET)
    endif()
endmacro()

# Wraps find_package with a colored status message.
#   frost_ext_find(LABEL package_name [extra_args...])
# The result variable <package_name>_FOUND is set by find_package as usual.
macro(frost_ext_find LABEL)
    find_package(${ARGN} ${_REQ})
    set(_FROST_EXT_FIND_ARGS ${ARGN})
    list(GET _FROST_EXT_FIND_ARGS 0 _PKG_NAME)
    if(${_PKG_NAME}_FOUND)
        message(STATUS "  ${FROST_GREEN}${LABEL}: found${FROST_RESET}")
    else()
        message(STATUS "  ${FROST_RED}${LABEL}: not found${FROST_RESET}")
    endif()
endmacro()

# Wraps pkg_check_modules with a colored status message.
#   frost_ext_pkg(LABEL var_name pkg_names...)
# The result variable <var_name>_FOUND is set by pkg_check_modules as usual.
macro(frost_ext_pkg LABEL VAR_NAME)
    pkg_check_modules(${VAR_NAME} ${_REQ} IMPORTED_TARGET ${ARGN})
    if(${VAR_NAME}_FOUND)
        message(STATUS "  ${FROST_GREEN}${LABEL}: found${FROST_RESET}")
    else()
        message(STATUS "  ${FROST_RED}${LABEL}: not found${FROST_RESET}")
    endif()
endmacro()

# Bail from an extension under AUTO mode. Under ON mode, emit a fatal error.
#   frost_ext_bail(message)
macro(frost_ext_bail MSG)
    if(FROST_EXT_MODE STREQUAL "ON")
        message(FATAL_ERROR "${MSG}")
    else()
        set(FROST_EXT_UNAVAILABLE TRUE PARENT_SCOPE)
        return()
    endif()
endmacro()

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

