set(INTEGRATION_TEST_DIR "${CMAKE_CURRENT_SOURCE_DIR}")
set(INTEGRATION_SCRIPTS
    smoke.frst
    prelude-inv.frst
    prelude-reject.frst
    prelude-curry.frst
    prelude-collect.frst
    prelude-call.frst
    ufcs-chain.frst
    capture-enclosing-closure.frst
)
foreach(script ${INTEGRATION_SCRIPTS})
    get_filename_component(name ${script} NAME_WE)
    add_test(
        NAME Frost_Integration_${name}
        COMMAND "${INTEGRATION_TEST_DIR}/runner.lua" $<TARGET_FILE:frost> "${INTEGRATION_TEST_DIR}/${script}"
    )
endforeach()
