set(HTTP_TEST_DIR "${CMAKE_CURRENT_SOURCE_DIR}")

add_test(
    NAME Frost_Integration_httpbin_setup
    COMMAND "${HTTP_TEST_DIR}/httpbin-setup.sh"
)
set_tests_properties(Frost_Integration_httpbin_setup PROPERTIES
    FIXTURES_SETUP httpbin
    SKIP_RETURN_CODE 77
)

add_test(
    NAME Frost_Integration_httpbin_get
    COMMAND "${HTTP_TEST_DIR}/httpbin-runner.sh"
            $<TARGET_FILE:frost>
            "${HTTP_TEST_DIR}/httpbin-get.frst"
)
set_tests_properties(Frost_Integration_httpbin_get PROPERTIES
    FIXTURES_REQUIRED httpbin
)

add_test(
    NAME Frost_Integration_httpbin_query_params
    COMMAND "${HTTP_TEST_DIR}/httpbin-runner.sh"
            $<TARGET_FILE:frost>
            "${HTTP_TEST_DIR}/httpbin-query-params.frst"
)
set_tests_properties(Frost_Integration_httpbin_query_params PROPERTIES
    FIXTURES_REQUIRED httpbin
)

add_test(
    NAME Frost_Integration_httpbin_headers
    COMMAND "${HTTP_TEST_DIR}/httpbin-runner.sh"
            $<TARGET_FILE:frost>
            "${HTTP_TEST_DIR}/httpbin-headers.frst"
)
set_tests_properties(Frost_Integration_httpbin_headers PROPERTIES
    FIXTURES_REQUIRED httpbin
)

add_test(
    NAME Frost_Integration_httpbin_post
    COMMAND "${HTTP_TEST_DIR}/httpbin-runner.sh"
            $<TARGET_FILE:frost>
            "${HTTP_TEST_DIR}/httpbin-post.frst"
)
set_tests_properties(Frost_Integration_httpbin_post PROPERTIES
    FIXTURES_REQUIRED httpbin
)

add_test(
    NAME Frost_Integration_httpbin_post_binary
    COMMAND "${HTTP_TEST_DIR}/httpbin-runner.sh"
            $<TARGET_FILE:frost>
            "${HTTP_TEST_DIR}/httpbin-post-binary.frst"
)
set_tests_properties(Frost_Integration_httpbin_post_binary PROPERTIES
    FIXTURES_REQUIRED httpbin
)

add_test(
    NAME Frost_Integration_httpbin_status_204
    COMMAND "${HTTP_TEST_DIR}/httpbin-runner.sh"
            $<TARGET_FILE:frost>
            "${HTTP_TEST_DIR}/httpbin-status-204.frst"
)
set_tests_properties(Frost_Integration_httpbin_status_204 PROPERTIES
    FIXTURES_REQUIRED httpbin
)

add_test(
    NAME Frost_Integration_httpbin_bytes_1024
    COMMAND "${HTTP_TEST_DIR}/httpbin-runner.sh"
            $<TARGET_FILE:frost>
            "${HTTP_TEST_DIR}/httpbin-bytes-1024.frst"
)
set_tests_properties(Frost_Integration_httpbin_bytes_1024 PROPERTIES
    FIXTURES_REQUIRED httpbin
)

add_test(
    NAME Frost_Integration_httpbin_put_delete
    COMMAND "${HTTP_TEST_DIR}/httpbin-runner.sh"
            $<TARGET_FILE:frost>
            "${HTTP_TEST_DIR}/httpbin-put-delete.frst"
)
set_tests_properties(Frost_Integration_httpbin_put_delete PROPERTIES
    FIXTURES_REQUIRED httpbin
)

add_test(
    NAME Frost_Integration_httpbin_timeout
    COMMAND "${HTTP_TEST_DIR}/httpbin-runner.sh"
            $<TARGET_FILE:frost>
            "${HTTP_TEST_DIR}/httpbin-timeout.frst"
)
set_tests_properties(Frost_Integration_httpbin_timeout PROPERTIES
    FIXTURES_REQUIRED httpbin
)

add_test(
    NAME Frost_Integration_httpbin_https_get
    COMMAND "${HTTP_TEST_DIR}/httpbin-https-runner.sh"
            $<TARGET_FILE:frost>
            "${HTTP_TEST_DIR}/httpbin-https-get.frst"
)
set_tests_properties(Frost_Integration_httpbin_https_get PROPERTIES
    SKIP_RETURN_CODE 77
)

add_test(
    NAME Frost_Integration_http_invalid_input
    COMMAND $<TARGET_FILE:frost> "${HTTP_TEST_DIR}/http-invalid-input.frst"
)

add_test(
    NAME Frost_Integration_httpbin_cleanup
    COMMAND "${HTTP_TEST_DIR}/httpbin-cleanup.sh"
)
set_tests_properties(Frost_Integration_httpbin_cleanup PROPERTIES
    FIXTURES_CLEANUP httpbin
)
