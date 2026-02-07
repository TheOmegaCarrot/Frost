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
    NAME Frost_Integration_httpbin_post
    COMMAND "${HTTP_TEST_DIR}/httpbin-runner.sh"
            $<TARGET_FILE:frost>
            "${HTTP_TEST_DIR}/httpbin-post.frst"
)
set_tests_properties(Frost_Integration_httpbin_post PROPERTIES
    FIXTURES_REQUIRED httpbin
)

add_test(
    NAME Frost_Integration_httpbin_cleanup
    COMMAND "${HTTP_TEST_DIR}/httpbin-cleanup.sh"
)
set_tests_properties(Frost_Integration_httpbin_cleanup PROPERTIES
    FIXTURES_CLEANUP httpbin
)
