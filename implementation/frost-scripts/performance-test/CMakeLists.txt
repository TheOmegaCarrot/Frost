find_program(PERF_EXECUTABLE perf)
find_program(VALGRIND_EXECUTABLE valgrind)
find_program(CALLGRIND_ANNOTATE_EXECUTABLE callgrind_annotate)

if(NOT PERF_EXECUTABLE
   AND (NOT VALGRIND_EXECUTABLE OR NOT CALLGRIND_ANNOTATE_EXECUTABLE))
    return()
endif()

set(FROST_PERF_STAT_REPEATS 7 CACHE STRING
    "Number of repeated runs for perf stat profiling")

set(
    PROFILING_TEST_SCRIPTS
    "${CMAKE_CURRENT_LIST_DIR}/array-concat-reduce-workload.frst"
    "${CMAKE_CURRENT_LIST_DIR}/closure-arg-bind-workload.frst"
    "${CMAKE_CURRENT_LIST_DIR}/closure-call-workload.frst"
    "${CMAKE_CURRENT_LIST_DIR}/closure-capture-workload.frst"
    "${CMAKE_CURRENT_LIST_DIR}/fib-map-reduce.frst"
    "${CMAKE_CURRENT_LIST_DIR}/map-filter-reduce.frst"
    "${CMAKE_CURRENT_LIST_DIR}/map-key-heavy-workload.frst"
    "${CMAKE_CURRENT_LIST_DIR}/map-union-workload.frst"
    "${CMAKE_CURRENT_LIST_DIR}/mixed-pipeline-workload.frst"
    "${CMAKE_CURRENT_LIST_DIR}/parse-stress-generated-literals.frst"
    "${CMAKE_CURRENT_LIST_DIR}/recursive-tailish-workload.frst"
    "${CMAKE_CURRENT_LIST_DIR}/string-accumulate-workload.frst"
    "${CMAKE_CURRENT_LIST_DIR}/string-transform-workload.frst"
    "${CMAKE_CURRENT_LIST_DIR}/symbol-lookup-workload.frst"
)

set(PERF_TARGETS)
set(PERF_STAT_TARGETS)
set(CALLGRIND_TARGETS)

foreach(script_path IN LISTS PROFILING_TEST_SCRIPTS)
    get_filename_component(script_name "${script_path}" NAME_WE)

    if(PERF_EXECUTABLE)
        set(script_out_dir "${CMAKE_BINARY_DIR}/profiling/${script_name}/perf")
        set(script_perf_file "${script_out_dir}/${script_name}.perf")
        set(script_report_self_file "${script_out_dir}/${script_name}.top.txt")
        set(script_report_inclusive_file
            "${script_out_dir}/${script_name}.top.inclusive.txt")
        set(script_stat_file "${script_out_dir}/${script_name}.stat.txt")

        add_custom_command(
            OUTPUT "${script_report_self_file}" "${script_report_inclusive_file}"
            BYPRODUCTS "${script_perf_file}"
            COMMAND "${CMAKE_COMMAND}" -E make_directory "${script_out_dir}"
            COMMAND "${PERF_EXECUTABLE}"
                    record
                    -q
                    -e cycles:u
                    -F 999
                    -g
                    --call-graph fp
                    -o "${script_perf_file}"
                    -- "$<TARGET_FILE:frost>" "${script_path}"
            COMMAND "${CMAKE_COMMAND}"
                    -DPERF_EXECUTABLE=${PERF_EXECUTABLE}
                    -DPERF_INPUT=${script_perf_file}
                    -DPERF_OUTPUT=${script_report_self_file}
                    -DPERF_INCLUDE_CHILDREN=OFF
                    -DPERF_SORT=dso,symbol
                    -DPERF_PERCENT_LIMIT=0.1
                    -P "${CMAKE_CURRENT_LIST_DIR}/write-perf-report.cmake"
            COMMAND "${CMAKE_COMMAND}"
                    -DPERF_EXECUTABLE=${PERF_EXECUTABLE}
                    -DPERF_INPUT=${script_perf_file}
                    -DPERF_OUTPUT=${script_report_inclusive_file}
                    -DPERF_INCLUDE_CHILDREN=ON
                    -DPERF_SORT=dso,symbol
                    -DPERF_PERCENT_LIMIT=0.1
                    -P "${CMAKE_CURRENT_LIST_DIR}/write-perf-report.cmake"
            DEPENDS
                frost
                "${script_path}"
                "${CMAKE_CURRENT_LIST_DIR}/write-perf-report.cmake"
            COMMENT "Generating perf artifacts for ${script_name}.frst"
            VERBATIM
        )

        set(target_name "perf-${script_name}")
        add_custom_target(
            "${target_name}"
            DEPENDS "${script_report_self_file}" "${script_report_inclusive_file}"
        )
        list(APPEND PERF_TARGETS "${target_name}")

        add_custom_command(
            OUTPUT "${script_stat_file}"
            COMMAND "${CMAKE_COMMAND}" -E make_directory "${script_out_dir}"
            COMMAND "${CMAKE_COMMAND}"
                    -DPERF_EXECUTABLE=${PERF_EXECUTABLE}
                    -DPERF_BINARY=$<TARGET_FILE:frost>
                    -DPERF_SCRIPT=${script_path}
                    -DPERF_OUTPUT=${script_stat_file}
                    -DPERF_REPEAT=${FROST_PERF_STAT_REPEATS}
                    -P "${CMAKE_CURRENT_LIST_DIR}/write-perf-stat.cmake"
            DEPENDS
                frost
                "${script_path}"
                "${CMAKE_CURRENT_LIST_DIR}/write-perf-stat.cmake"
            COMMENT "Generating perf stat artifacts for ${script_name}.frst"
            VERBATIM
        )

        set(target_name "perf-stat-${script_name}")
        add_custom_target("${target_name}" DEPENDS "${script_stat_file}")
        list(APPEND PERF_STAT_TARGETS "${target_name}")
    endif()

    if(VALGRIND_EXECUTABLE AND CALLGRIND_ANNOTATE_EXECUTABLE)
        set(script_out_dir
            "${CMAKE_BINARY_DIR}/profiling/${script_name}/callgrind")
        set(script_callgrind_file "${script_out_dir}/${script_name}.callgrind")
        set(script_report_self_file
            "${script_out_dir}/${script_name}.annotate.self.txt")
        set(script_report_inclusive_file
            "${script_out_dir}/${script_name}.annotate.txt")
        set(script_log_file "${script_out_dir}/${script_name}.log")

        add_custom_command(
            OUTPUT "${script_report_self_file}" "${script_report_inclusive_file}"
            BYPRODUCTS "${script_callgrind_file}" "${script_log_file}"
            COMMAND "${CMAKE_COMMAND}" -E make_directory "${script_out_dir}"
            COMMAND "${VALGRIND_EXECUTABLE}"
                    --tool=callgrind
                    --cache-sim=no
                    --branch-sim=no
                    --callgrind-out-file=${script_callgrind_file}
                    --log-file=${script_log_file}
                    -- "$<TARGET_FILE:frost>" "${script_path}"
            COMMAND "${CMAKE_COMMAND}"
                    -DCALLGRIND_ANNOTATE_EXECUTABLE=${CALLGRIND_ANNOTATE_EXECUTABLE}
                    -DCALLGRIND_INPUT=${script_callgrind_file}
                    -DCALLGRIND_OUTPUT=${script_report_self_file}
                    -DCALLGRIND_INCLUSIVE=OFF
                    -P "${CMAKE_CURRENT_LIST_DIR}/write-callgrind-annotate.cmake"
            COMMAND "${CMAKE_COMMAND}"
                    -DCALLGRIND_ANNOTATE_EXECUTABLE=${CALLGRIND_ANNOTATE_EXECUTABLE}
                    -DCALLGRIND_INPUT=${script_callgrind_file}
                    -DCALLGRIND_OUTPUT=${script_report_inclusive_file}
                    -DCALLGRIND_INCLUSIVE=ON
                    -P "${CMAKE_CURRENT_LIST_DIR}/write-callgrind-annotate.cmake"
            DEPENDS
                frost
                "${script_path}"
                "${CMAKE_CURRENT_LIST_DIR}/write-callgrind-annotate.cmake"
            COMMENT "Generating callgrind artifacts for ${script_name}.frst"
            VERBATIM
        )

        set(target_name "callgrind-${script_name}")
        add_custom_target(
            "${target_name}"
            DEPENDS "${script_report_self_file}" "${script_report_inclusive_file}"
        )
        list(APPEND CALLGRIND_TARGETS "${target_name}")
    endif()
endforeach()

if(PERF_TARGETS)
    add_custom_target(perf-all DEPENDS ${PERF_TARGETS})
endif()

if(PERF_STAT_TARGETS)
    add_custom_target(perf-stat-all DEPENDS ${PERF_STAT_TARGETS})
endif()

if(CALLGRIND_TARGETS)
    add_custom_target(callgrind-all DEPENDS ${CALLGRIND_TARGETS})
endif()

set(PROFILE_TARGETS
    ${PERF_TARGETS}
    ${PERF_STAT_TARGETS}
    ${CALLGRIND_TARGETS}
)

if(PROFILE_TARGETS)
    # Serialize profiling runs while keeping normal builds parallelizable.
    add_custom_target(profile)

    set(previous_profile_step "")
    set(profile_step_index 0)
    foreach(profiling_target IN LISTS PROFILE_TARGETS)
        math(EXPR profile_step_index "${profile_step_index} + 1")
        set(profile_step_target "profile-step-${profile_step_index}")
        add_custom_target("${profile_step_target}" DEPENDS "${profiling_target}")

        if(previous_profile_step)
            add_dependencies("${profile_step_target}" "${previous_profile_step}")
        endif()

        set(previous_profile_step "${profile_step_target}")
    endforeach()

    add_dependencies(profile "${previous_profile_step}")
endif()
