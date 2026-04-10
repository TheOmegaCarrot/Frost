set(FROST_SCRIPTS_TEST_DIR "${CMAKE_CURRENT_SOURCE_DIR}")
set(FROST_SCRIPTS_RUNNER "${CMAKE_CURRENT_SOURCE_DIR}/../runner.lua")
set(FROST_SCRIPTS_MODULE_PATH "${CMAKE_SOURCE_DIR}/frost-scripts")

# Root utility tests — individual scripts that test exported functions
set(FROST_SCRIPTS_TESTS
    test-fib.frst
    test-lazy.frst
    test-object.frst
    test-re.frst
)
foreach(script ${FROST_SCRIPTS_TESTS})
    get_filename_component(name ${script} NAME_WE)
    add_test(
        NAME Frost_Integration_FrostScripts_${name}
        COMMAND "${FROST_SCRIPTS_RUNNER}" $<TARGET_FILE:frost>
                "${FROST_SCRIPTS_TEST_DIR}/${script}"
    )
    set_tests_properties(
        Frost_Integration_FrostScripts_${name}
        PROPERTIES ENVIRONMENT
            "FROST_MODULE_PATH=${FROST_SCRIPTS_MODULE_PATH}"
    )
endforeach()

# Solution scripts — import and run their built-in assertions
set(FROST_SCRIPTS_SOLUTIONS
    example-problems.1-order-analytics-solution
    example-problems.2-recursive-flatten-solution
    example-problems.3-rpn-calculator-solution
    example-problems.4-deep-config-merge-solution
    example-problems.5-closure-counter-solution
    example-problems.6-file-log-summarizer-solution
    example-problems.7-regex-named-groups-solution
    example-problems.8-prime-count-stress-solution
    example-problems.9-cli-args-router-solution
)
foreach(module ${FROST_SCRIPTS_SOLUTIONS})
    string(REPLACE "." "-" name ${module})
    add_test(
        NAME Frost_Integration_FrostScripts_${name}
        COMMAND $<TARGET_FILE:frost>
                "${FROST_SCRIPTS_TEST_DIR}/import-runner.frst"
                ${module}
    )
    set_tests_properties(
        Frost_Integration_FrostScripts_${name}
        PROPERTIES ENVIRONMENT
            "FROST_MODULE_PATH=${FROST_SCRIPTS_MODULE_PATH}"
    )
endforeach()
