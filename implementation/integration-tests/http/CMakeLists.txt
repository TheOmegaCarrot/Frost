set(HTTP_TEST_DIR "${CMAKE_CURRENT_SOURCE_DIR}")

function(add_http_script_test)
    set(options REQUIRES_HTTPBIN)
    set(one_value_args RUNNER SCRIPT)
    cmake_parse_arguments(HTTP_TEST "${options}" "${one_value_args}" "" ${ARGN})

    get_filename_component(test_stem "${HTTP_TEST_SCRIPT}" NAME_WE)
    string(REPLACE "-" "_" test_stem "${test_stem}")
    set(test_name "Frost_Integration_${test_stem}")

    add_test(
        NAME ${test_name}
        COMMAND "${HTTP_TEST_DIR}/${HTTP_TEST_RUNNER}"
                $<TARGET_FILE:frost>
                "${HTTP_TEST_DIR}/${HTTP_TEST_SCRIPT}"
    )

    if(HTTP_TEST_REQUIRES_HTTPBIN)
        set_tests_properties(${test_name} PROPERTIES
            FIXTURES_REQUIRED httpbin
        )
    endif()
endfunction()

add_test(
    NAME Frost_Integration_httpbin_setup
    COMMAND "${HTTP_TEST_DIR}/httpbin-setup.sh"
)
set_tests_properties(Frost_Integration_httpbin_setup PROPERTIES
    FIXTURES_SETUP httpbin
    SKIP_RETURN_CODE 77
)

set(HTTPBIN_FIXTURE_SCRIPTS
    httpbin-get.frst
    httpbin-get-memoization.frst
    httpbin-query-params.frst
    httpbin-outbound-multivalue.frst
    httpbin-headers.frst
    httpbin-response-headers.frst
    httpbin-head.frst
    httpbin-path-normalization.frst
    httpbin-post.frst
    httpbin-post-binary.frst
    httpbin-status-204.frst
    httpbin-status-500-ok.frst
    httpbin-bytes-1024.frst
    httpbin-put-delete.frst
    httpbin-timeout.frst
    httpbin-tls-knobs-plain-http.frst
    http-error-taxonomy.frst
)

foreach(script ${HTTPBIN_FIXTURE_SCRIPTS})
    add_http_script_test(
        RUNNER httpbin-runner.sh
        SCRIPT "${script}"
        REQUIRES_HTTPBIN
    )
endforeach()

set(HTTPS_HTTPBIN_RUNNER_SCRIPTS
    http-endpoint-defaults.frst
    httpbin-https-get.frst
)

foreach(script ${HTTPS_HTTPBIN_RUNNER_SCRIPTS})
    add_http_script_test(
        RUNNER httpbin-https-runner.sh
        SCRIPT "${script}"
    )
endforeach()

add_http_script_test(
    RUNNER https-verify-toggle-runner.sh
    SCRIPT https-verify-toggle.frst
)

add_http_script_test(
    RUNNER https-ca-controls-runner.sh
    SCRIPT https-ca-controls.frst
)

set(HTTP_DIRECT_SCRIPTS
    http-invalid-input.frst
    http-invalid-parse-matrix.frst
    http-invalid-parse-guards-extra.frst
)

foreach(script ${HTTP_DIRECT_SCRIPTS})
    add_http_script_test(
        RUNNER http-direct-runner.sh
        SCRIPT "${script}"
    )
endforeach()

add_test(
    NAME Frost_Integration_httpbin_cleanup
    COMMAND "${HTTP_TEST_DIR}/httpbin-cleanup.sh"
)
set_tests_properties(Frost_Integration_httpbin_cleanup PROPERTIES
    FIXTURES_CLEANUP httpbin
)

get_property(_http_tests DIRECTORY PROPERTY TESTS)
set_tests_properties(${_http_tests} PROPERTIES
    SKIP_RETURN_CODE 77
)
