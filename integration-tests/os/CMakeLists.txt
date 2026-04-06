set(OS_TEST_DIR "${CMAKE_CURRENT_SOURCE_DIR}")
set(OS_RUNNER "${CMAKE_CURRENT_SOURCE_DIR}/../runner.lua")
set(OS_SCRIPTS
    run-basic.frst
    run-stdin.frst
    run-cwd.frst
    run-env.frst
    run-errors.frst
)
foreach(script ${OS_SCRIPTS})
    get_filename_component(name ${script} NAME_WE)
    add_test(
        NAME Frost_Integration_OS_${name}
        COMMAND "${OS_RUNNER}" $<TARGET_FILE:frost> "${OS_TEST_DIR}/${script}"
    )
endforeach()
