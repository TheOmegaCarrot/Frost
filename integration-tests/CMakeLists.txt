set(INTEGRATION_TEST_DIR "${CMAKE_CURRENT_SOURCE_DIR}")
set(INTEGRATION_SCRIPTS
    smoke.frst
    smoke-2.frst
    prelude/inv.frst
    prelude/reject.frst
    prelude/curry.frst
    prelude/collect.frst
    prelude/call.frst
    prelude/rev-args.frst
    prelude/tap.frst
    prelude/const.frst
    prelude/compose.frst
    prelude/index.frst
    prelude/map_keys_values.frst
    prelude/flat_map.frst
    prelude/dig.frst
    threaded-call-chain.frst
    capture-enclosing-closure.frst
    escaped-self-reference.frst
    map-newline-1.frst
)
foreach(script ${INTEGRATION_SCRIPTS})
    get_filename_component(name ${script} NAME_WE)
    add_test(
        NAME Frost_Integration_${name}
        COMMAND "${INTEGRATION_TEST_DIR}/runner.lua" $<TARGET_FILE:frost> "${INTEGRATION_TEST_DIR}/${script}"
    )
endforeach()

add_subdirectory(import)
add_subdirectory(backtrace)
add_subdirectory(frost-scripts)
