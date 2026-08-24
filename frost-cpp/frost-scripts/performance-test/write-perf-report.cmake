if(NOT DEFINED PERF_EXECUTABLE)
    message(FATAL_ERROR "PERF_EXECUTABLE is required")
endif()

if(NOT DEFINED PERF_INPUT)
    message(FATAL_ERROR "PERF_INPUT is required")
endif()

if(NOT DEFINED PERF_OUTPUT)
    message(FATAL_ERROR "PERF_OUTPUT is required")
endif()

if(NOT DEFINED PERF_INCLUDE_CHILDREN)
    set(PERF_INCLUDE_CHILDREN OFF)
endif()

if(NOT DEFINED PERF_SORT)
    set(PERF_SORT symbol)
endif()

if(NOT DEFINED PERF_PERCENT_LIMIT)
    set(PERF_PERCENT_LIMIT 0.5)
endif()

set(perf_report_args
    report
    --stdio
    --sort ${PERF_SORT}
    --percent-limit ${PERF_PERCENT_LIMIT}
    -i "${PERF_INPUT}"
)

if(PERF_INCLUDE_CHILDREN)
    list(APPEND perf_report_args --children)
else()
    list(APPEND perf_report_args --no-children)
endif()

execute_process(
    COMMAND "${PERF_EXECUTABLE}" ${perf_report_args}
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
