add_executable(crash-after-flush helpers/crash-after-flush.cpp)
add_executable(sigpipe-self helpers/sigpipe-self.cpp)

set(OS_TEST_DIR "${CMAKE_CURRENT_SOURCE_DIR}")
set(OS_RUNNER "${CMAKE_CURRENT_SOURCE_DIR}/../runner.lua")
set(OS_SCRIPTS
    run-basic.frst
    run-stdin.frst
    run-cwd.frst
    run-env.frst
    run-errors.frst
    run-streams.frst
    run-stdin-edge.frst
    run-stdin-reread.frst
    run-concurrent-streams.frst
    run-large-stdin.frst
)
foreach(script ${OS_SCRIPTS})
    get_filename_component(name ${script} NAME_WE)
    add_test(
        NAME Frost_Integration_OS_${name}
        COMMAND "${OS_RUNNER}" $<TARGET_FILE:frost> "${OS_TEST_DIR}/${script}"
    )
endforeach()

# Signal test: call frost directly (not via runner) to pass helper paths as script args
add_test(
    NAME Frost_Integration_OS_run-signals
    COMMAND $<TARGET_FILE:frost> "${OS_TEST_DIR}/run-signals.frst"
            $<TARGET_FILE:crash-after-flush> $<TARGET_FILE:sigpipe-self>
)
