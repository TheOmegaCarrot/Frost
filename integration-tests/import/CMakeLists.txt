# AI-generated
set(IMPORT_TEST_DIR "${CMAKE_CURRENT_SOURCE_DIR}")
set(IMPORT_RUNNER "${CMAKE_CURRENT_SOURCE_DIR}/../runner.lua")
set(IMPORT_SCRIPTS
    import-local.frst
    import-nested.frst
    import-env.frst
    import-errors.frst
    import-first-match.frst
    import-null-export.frst
    import-parse-error.frst
    import-cwd.frst
    import-parent-cwd.frst
    import-destructure.frst
    import-deep.frst
)
foreach(script ${IMPORT_SCRIPTS})
    get_filename_component(name ${script} NAME_WE)
    add_test(
        NAME Frost_Integration_Import_${name}
        COMMAND "${IMPORT_RUNNER}" $<TARGET_FILE:frost> "${IMPORT_TEST_DIR}/${script}"
    )
endforeach()

set_tests_properties(
    Frost_Integration_Import_import-env
    PROPERTIES ENVIRONMENT "FROST_MODULE_PATH=${IMPORT_TEST_DIR}/envmods"
)

set_tests_properties(
    Frost_Integration_Import_import-first-match
    PROPERTIES ENVIRONMENT "FROST_MODULE_PATH=${IMPORT_TEST_DIR}/envmods"
)

set_tests_properties(
    Frost_Integration_Import_import-cwd
    PROPERTIES WORKING_DIRECTORY "${IMPORT_TEST_DIR}/cwdroot"
)

set_tests_properties(
    Frost_Integration_Import_import-parent-cwd
    PROPERTIES WORKING_DIRECTORY "${IMPORT_TEST_DIR}/cwdroot"
)
