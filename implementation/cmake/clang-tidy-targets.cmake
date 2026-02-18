include_guard(GLOBAL)

find_program(CLANG_TIDY_EXECUTABLE clang-tidy)

if(NOT CLANG_TIDY_EXECUTABLE)
    message(STATUS "clang-tidy not found; check targets will not be available")
    set(FROST_CLANG_TIDY_AVAILABLE OFF CACHE INTERNAL "")
else()
    set(FROST_CLANG_TIDY_AVAILABLE ON CACHE INTERNAL "")
    message(STATUS "Using clang-tidy: ${CLANG_TIDY_EXECUTABLE}")
endif()

set(FROST_CLANG_TIDY_HEADER_FILTER
    "^${CMAKE_SOURCE_DIR}/cpp/"
    CACHE STRING
    "Regex for headers that clang-tidy should report diagnostics from"
)
set(FROST_CLANG_TIDY_EXCLUDE_HEADER_FILTER
    "^${CMAKE_BINARY_DIR}/_deps/|^${CMAKE_SOURCE_DIR}/external/"
    CACHE STRING
    "Regex for headers clang-tidy should suppress diagnostics from"
)

function(frost_collect_target_sources out_var)
    set(options EXCLUDE_TESTS)
    set(one_value_args)
    set(multi_value_args TARGETS)
    cmake_parse_arguments(FROST_COLLECT "${options}" "${one_value_args}"
                          "${multi_value_args}" ${ARGN})

    set(collected_sources)
    foreach(target IN LISTS FROST_COLLECT_TARGETS)
        if(NOT TARGET ${target})
            continue()
        endif()

        get_target_property(target_sources ${target} SOURCES)
        if(NOT target_sources OR target_sources STREQUAL "target_sources-NOTFOUND")
            continue()
        endif()

        get_target_property(target_source_dir ${target} SOURCE_DIR)

        foreach(source IN LISTS target_sources)
            if(source MATCHES "^\\$<")
                continue()
            endif()

            if(NOT source MATCHES "\\.cpp$")
                continue()
            endif()

            if(IS_ABSOLUTE "${source}")
                set(abs_source "${source}")
            else()
                set(abs_source "${target_source_dir}/${source}")
            endif()

            if(NOT EXISTS "${abs_source}")
                continue()
            endif()

            if(FROST_COLLECT_EXCLUDE_TESTS AND abs_source MATCHES "/tests/")
                continue()
            endif()

            list(APPEND collected_sources "${abs_source}")
        endforeach()
    endforeach()

    list(REMOVE_DUPLICATES collected_sources)
    set(${out_var} "${collected_sources}" PARENT_SCOPE)
endfunction()

function(frost_create_clang_tidy_target target_name)
    if(NOT FROST_CLANG_TIDY_AVAILABLE)
        return()
    endif()

    set(options)
    set(one_value_args GROUP_LABEL)
    set(multi_value_args SOURCES)
    cmake_parse_arguments(FROST_TIDY "${options}" "${one_value_args}"
                          "${multi_value_args}" ${ARGN})

    if(NOT FROST_TIDY_GROUP_LABEL)
        set(FROST_TIDY_GROUP_LABEL "${target_name}")
    endif()

    set(normalized_sources)
    foreach(source IN LISTS FROST_TIDY_SOURCES)
        if(NOT source MATCHES "\\.cpp$")
            continue()
        endif()

        if(IS_ABSOLUTE "${source}")
            set(abs_source "${source}")
        else()
            set(abs_source "${CMAKE_CURRENT_SOURCE_DIR}/${source}")
        endif()

        if(EXISTS "${abs_source}")
            list(APPEND normalized_sources "${abs_source}")
        endif()
    endforeach()

    list(REMOVE_DUPLICATES normalized_sources)

    if(NOT normalized_sources)
        add_custom_target(${target_name}
            COMMAND "${CMAKE_COMMAND}" -E echo "No sources registered for ${target_name}"
            VERBATIM
        )
        return()
    endif()

    set(stamps)
    foreach(source IN LISTS normalized_sources)
        string(MD5 source_hash "${source}")
        set(out_dir "${CMAKE_BINARY_DIR}/clang-tidy/${FROST_TIDY_GROUP_LABEL}")
        set(stamp "${out_dir}/${source_hash}.stamp")
        set(clang_tidy_depends "${source}")
        if(EXISTS "${CMAKE_SOURCE_DIR}/.clang-tidy")
            list(APPEND clang_tidy_depends "${CMAKE_SOURCE_DIR}/.clang-tidy")
        endif()

        add_custom_command(
            OUTPUT "${stamp}"
            COMMAND "${CMAKE_COMMAND}" -E make_directory "${out_dir}"
            COMMAND "${CLANG_TIDY_EXECUTABLE}"
                    -p "${CMAKE_BINARY_DIR}"
                    "--header-filter=${FROST_CLANG_TIDY_HEADER_FILTER}"
                    "--exclude-header-filter=${FROST_CLANG_TIDY_EXCLUDE_HEADER_FILTER}"
                    "${source}"
            COMMAND "${CMAKE_COMMAND}" -E touch "${stamp}"
            DEPENDS ${clang_tidy_depends}
            COMMENT "Running clang-tidy on ${source}"
            VERBATIM
        )
        list(APPEND stamps "${stamp}")
    endforeach()

    add_custom_target(${target_name} DEPENDS ${stamps})
endfunction()
