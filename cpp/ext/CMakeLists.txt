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

# Register a Frost extension.
# For each NAME in the list:
#   - declares option WITH_<UPPER_NAME> (default ON)
#   - if enabled, adds the subdirectory, links frost-<name>, and defines FROST_ENABLE_<UPPER_NAME>
macro(frost_extension NAME)
    string(TOUPPER ${NAME} _UPPER)
    option(WITH_${_UPPER} "Enable ${NAME} extension" ON)
    if(WITH_${_UPPER})
        message(STATUS "Enabling ${NAME}")
        add_subdirectory(${NAME})
        target_link_libraries(frost-ext PRIVATE frost-${NAME})
        target_compile_definitions(frost-ext PRIVATE FROST_ENABLE_${_UPPER})
    endif()
endmacro()

frost_extension(hash)
frost_extension(http)
frost_extension(sqlite)
frost_extension(compression)
frost_extension(unsafe)

