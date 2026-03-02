if(NOT DEFINED PERF_EXECUTABLE)
    message(FATAL_ERROR "PERF_EXECUTABLE is required")
endif()

if(NOT DEFINED PERF_BINARY)
    message(FATAL_ERROR "PERF_BINARY is required")
endif()

if(NOT DEFINED PERF_SCRIPT)
    message(FATAL_ERROR "PERF_SCRIPT is required")
endif()

if(NOT DEFINED PERF_OUTPUT)
    message(FATAL_ERROR "PERF_OUTPUT is required")
endif()

if(NOT DEFINED PERF_REPEAT)
    set(PERF_REPEAT 7)
endif()

execute_process(
    COMMAND "${PERF_EXECUTABLE}"
            stat
            -r "${PERF_REPEAT}"
            -d
            -- "${PERF_BINARY}" "${PERF_SCRIPT}"
    OUTPUT_QUIET
    ERROR_FILE "${PERF_OUTPUT}"
    RESULT_VARIABLE perf_stat_result
)

if(NOT perf_stat_result EQUAL 0)
    file(READ "${PERF_OUTPUT}" perf_stat_error)
    message(
        FATAL_ERROR
        "perf stat failed for ${PERF_SCRIPT}: ${perf_stat_error}"
    )
endif()
