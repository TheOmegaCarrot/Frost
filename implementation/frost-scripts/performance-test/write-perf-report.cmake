if(NOT DEFINED PERF_EXECUTABLE)
    message(FATAL_ERROR "PERF_EXECUTABLE is required")
endif()

if(NOT DEFINED PERF_INPUT)
    message(FATAL_ERROR "PERF_INPUT is required")
endif()

if(NOT DEFINED PERF_OUTPUT)
    message(FATAL_ERROR "PERF_OUTPUT is required")
endif()

execute_process(
    COMMAND "${PERF_EXECUTABLE}"
            report
            --stdio
            --no-children
            --sort symbol
            --percent-limit 0.5
            -i "${PERF_INPUT}"
    OUTPUT_FILE "${PERF_OUTPUT}"
    ERROR_VARIABLE perf_report_error
    RESULT_VARIABLE perf_report_result
)

if(NOT perf_report_result EQUAL 0)
    message(
        FATAL_ERROR
        "perf report failed for ${PERF_INPUT}: ${perf_report_error}"
    )
endif()
