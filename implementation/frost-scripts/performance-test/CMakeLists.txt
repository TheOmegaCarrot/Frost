find_program(PERF_EXECUTABLE perf)
if(NOT PERF_EXECUTABLE)
    return()
endif()

set(
    PERF_TEST_SCRIPTS
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

foreach(script_path IN LISTS PERF_TEST_SCRIPTS)
    get_filename_component(script_name "${script_path}" NAME_WE)

    set(script_out_dir "${CMAKE_BINARY_DIR}/profile/performance-test/${script_name}")
    set(script_perf_file "${script_out_dir}/${script_name}.perf")
    set(script_report_file "${script_out_dir}/${script_name}.top.txt")

    add_custom_command(
        OUTPUT "${script_report_file}"
        BYPRODUCTS "${script_perf_file}"
        COMMAND "${CMAKE_COMMAND}" -E make_directory "${script_out_dir}"
        COMMAND "${PERF_EXECUTABLE}"
                record
                -q
                --all-user
                -o "${script_perf_file}"
                -- "$<TARGET_FILE:frost>" "${script_path}"
        COMMAND "${CMAKE_COMMAND}"
                -DPERF_EXECUTABLE=${PERF_EXECUTABLE}
                -DPERF_INPUT=${script_perf_file}
                -DPERF_OUTPUT=${script_report_file}
                -P "${CMAKE_CURRENT_LIST_DIR}/write-perf-report.cmake"
        DEPENDS
            frost
            "${script_path}"
            "${CMAKE_CURRENT_LIST_DIR}/write-perf-report.cmake"
        COMMENT "Generating profiling artifacts for ${script_name}.frst"
        VERBATIM
    )

    set(target_name "perf-${script_name}")
    add_custom_target("${target_name}" DEPENDS "${script_report_file}")
    list(APPEND PERF_TARGETS "${target_name}")
endforeach()

if(PERF_TARGETS)
    add_custom_target(perf-all ALL DEPENDS ${PERF_TARGETS})
endif()
