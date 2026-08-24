set(BACKTRACE_TEST_DIR "${CMAKE_CURRENT_SOURCE_DIR}")
set(BACKTRACE_RUNNER "${CMAKE_CURRENT_SOURCE_DIR}/../runner.lua")

# Category 1: Crash tests - check stderr content
set(CRASH_SCRIPTS
    error-through-function-call.frst
    error-through-do-block.frst
    error-through-map.frst
    error-through-filter.frst
    error-through-reduce.frst
    error-through-foreach.frst
    error-nested-boundaries.frst
    error-arg-evaluation.frst
    error-recursive.frst
    error-threaded-call.frst
    error-nested-hof.frst
    error-through-builtin-hof.frst
    error-deeply-nested.frst
    error-non-boundary-frames.frst
    error-before-hof-boundary.frst
    error-calling-non-function.frst
    error-in-function-position.frst
    error-reduce-not-first.frst
    error-through-prelude.frst
    error-in-array-constructor.frst
    error-in-do-prefix.frst
    error-in-index.frst
    error-in-format-string.frst
    error-user-hof.frst
    error-destructure-array.frst
    error-destructure-nested-array.frst
    error-destructure-map.frst
    error-destructure-computed-key.frst
    error-destructure-map-in-array.frst
    error-destructure-deeply-nested.frst
)

foreach(script ${CRASH_SCRIPTS})
    get_filename_component(name ${script} NAME_WE)

    # With backtrace - check traced stderr
    add_test(
        NAME Frost_Integration_Backtrace_${name}
        COMMAND "${BACKTRACE_RUNNER}" $<TARGET_FILE:frost> "${BACKTRACE_TEST_DIR}/${script}"
                --expect "${BACKTRACE_TEST_DIR}/${script}.traced.stderr"
                -- --enable-backtrace
    )

    # Without backtrace - check plain stderr (no Traceback section)
    add_test(
        NAME Frost_Integration_Backtrace_no_trace_${name}
        COMMAND "${BACKTRACE_RUNNER}" $<TARGET_FILE:frost> "${BACKTRACE_TEST_DIR}/${script}"
                --expect "${BACKTRACE_TEST_DIR}/${script}.plain.stderr"
    )
endforeach()

# Category 2: Catch-and-continue - catch, verify clean state, then crash
set(CONTINUE_SCRIPTS
    catch-continue-through.frst
    catch-continue-after.frst
    sequential-catches.frst
    catch-deep-then-crash.frst
    catch-different-errors.frst
)

foreach(script ${CONTINUE_SCRIPTS})
    get_filename_component(name ${script} NAME_WE)

    # With backtrace - check final crash trace is clean
    add_test(
        NAME Frost_Integration_Backtrace_${name}
        COMMAND "${BACKTRACE_RUNNER}" $<TARGET_FILE:frost> "${BACKTRACE_TEST_DIR}/${script}"
                --expect "${BACKTRACE_TEST_DIR}/${script}.traced.stderr"
                -- --enable-backtrace
    )

    # Without backtrace - check final crash has no traceback
    add_test(
        NAME Frost_Integration_Backtrace_no_trace_${name}
        COMMAND "${BACKTRACE_RUNNER}" $<TARGET_FILE:frost> "${BACKTRACE_TEST_DIR}/${script}"
                --expect "${BACKTRACE_TEST_DIR}/${script}.plain.stderr"
    )
endforeach()

# Category 3: Import boundary tests - crash scripts imported through various styles
set(IMPORT_RUNNER "${BACKTRACE_TEST_DIR}/import-backtrace-runner.lua")
set(IMPORTER_DIR "${BACKTRACE_TEST_DIR}/importers")

set(IMPORTER_STYLES bare threaded in-do via-function)

foreach(style ${IMPORTER_STYLES})
    foreach(script ${CRASH_SCRIPTS})
        get_filename_component(module ${script} NAME_WE)

        # With backtrace - check inner trace + import suffix
        add_test(
            NAME Frost_Integration_Backtrace_import_${style}_${module}
            COMMAND "${IMPORT_RUNNER}" $<TARGET_FILE:frost>
                    "${IMPORTER_DIR}/${style}.frst"
                    "${BACKTRACE_TEST_DIR}" ${module}
                    --expect-inner "${BACKTRACE_TEST_DIR}/${script}.traced.stderr"
                    --expect-suffix "${IMPORTER_DIR}/${style}.traced.suffix"
                    -- --enable-backtrace
        )

        # Without backtrace - only the error message
        add_test(
            NAME Frost_Integration_Backtrace_import_no_trace_${style}_${module}
            COMMAND "${IMPORT_RUNNER}" $<TARGET_FILE:frost>
                    "${IMPORTER_DIR}/${style}.frst"
                    "${BACKTRACE_TEST_DIR}" ${module}
                    --expect-inner "${BACKTRACE_TEST_DIR}/${script}.plain.stderr"
        )
    endforeach()
endforeach()

# Category 4: Chained import tests (A imports B imports C, C crashes)
set(CHAIN_SCRIPTS chain-a.frst)

foreach(script ${CHAIN_SCRIPTS})
    get_filename_component(name ${script} NAME_WE)

    add_test(
        NAME Frost_Integration_Backtrace_${name}
        COMMAND "${BACKTRACE_RUNNER}" $<TARGET_FILE:frost> "${BACKTRACE_TEST_DIR}/${script}"
                --expect "${BACKTRACE_TEST_DIR}/${script}.traced.stderr"
                -- --enable-backtrace
    )
    set_tests_properties(Frost_Integration_Backtrace_${name} PROPERTIES
        ENVIRONMENT "FROST_MODULE_PATH=${BACKTRACE_TEST_DIR}"
    )

    add_test(
        NAME Frost_Integration_Backtrace_no_trace_${name}
        COMMAND "${BACKTRACE_RUNNER}" $<TARGET_FILE:frost> "${BACKTRACE_TEST_DIR}/${script}"
                --expect "${BACKTRACE_TEST_DIR}/${script}.plain.stderr"
    )
    set_tests_properties(Frost_Integration_Backtrace_no_trace_${name} PROPERTIES
        ENVIRONMENT "FROST_MODULE_PATH=${BACKTRACE_TEST_DIR}"
    )
endforeach()

# Category 5: try_call trace tests - assert-based, expect exit 0
add_test(
    NAME Frost_Integration_Backtrace_try_call_trace
    COMMAND "${BACKTRACE_RUNNER}" $<TARGET_FILE:frost> "${BACKTRACE_TEST_DIR}/try-call-trace.frst"
            -- --enable-backtrace
)

add_test(
    NAME Frost_Integration_Backtrace_try_call_no_trace
    COMMAND "${BACKTRACE_RUNNER}" $<TARGET_FILE:frost> "${BACKTRACE_TEST_DIR}/try-call-no-trace.frst"
)
