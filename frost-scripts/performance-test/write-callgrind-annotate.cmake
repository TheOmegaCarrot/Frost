if(NOT DEFINED CALLGRIND_ANNOTATE_EXECUTABLE)
    message(FATAL_ERROR "CALLGRIND_ANNOTATE_EXECUTABLE is required")
endif()

if(NOT DEFINED CALLGRIND_INPUT)
    message(FATAL_ERROR "CALLGRIND_INPUT is required")
endif()

if(NOT DEFINED CALLGRIND_OUTPUT)
    message(FATAL_ERROR "CALLGRIND_OUTPUT is required")
endif()

if(NOT DEFINED CALLGRIND_INCLUSIVE)
    set(CALLGRIND_INCLUSIVE OFF)
endif()

set(callgrind_inclusive_flag "--inclusive=no")
if(CALLGRIND_INCLUSIVE)
    set(callgrind_inclusive_flag "--inclusive=yes")
endif()

execute_process(
    COMMAND "${CALLGRIND_ANNOTATE_EXECUTABLE}"
            ${callgrind_inclusive_flag}
            --auto=no
            --show=Ir
            --sort=Ir
            "${CALLGRIND_INPUT}"
    OUTPUT_FILE "${CALLGRIND_OUTPUT}"
    ERROR_VARIABLE callgrind_annotate_error
    RESULT_VARIABLE callgrind_annotate_result
)

if(NOT callgrind_annotate_result EQUAL 0)
    message(
        FATAL_ERROR
        "callgrind_annotate failed for ${CALLGRIND_INPUT}: ${callgrind_annotate_error}"
    )
endif()
