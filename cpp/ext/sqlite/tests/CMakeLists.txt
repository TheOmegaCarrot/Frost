set(SQLITE_TEST_DIR "${CMAKE_CURRENT_SOURCE_DIR}")

set(SQLITE_TESTS
    basic-open-execute.frst
    basic-errors.frst
)

find_program(VALGRIND valgrind)

foreach(script ${SQLITE_TESTS})
    get_filename_component(name ${script} NAME_WE)
    set(db_path "${CMAKE_CURRENT_BINARY_DIR}/${name}.db")

    # In-memory
    add_test(
        NAME Frost_Ext_SQLite_${name}_memory
        COMMAND $<TARGET_FILE:frost>
                "${SQLITE_TEST_DIR}/${script}" -- :memory:
    )

    # Filesystem (each test gets its own db file to avoid lock conflicts)
    add_test(
        NAME Frost_Ext_SQLite_${name}_file
        COMMAND $<TARGET_FILE:frost>
                "${SQLITE_TEST_DIR}/${script}" -- ${db_path}
    )

    if(VALGRIND)
        # In-memory under Valgrind
        add_test(
            NAME Frost_Ext_SQLite_Valgrind_${name}_memory
            COMMAND ${VALGRIND} --error-exitcode=1 --leak-check=full
                    $<TARGET_FILE:frost>
                    "${SQLITE_TEST_DIR}/${script}" -- :memory:
        )

        # Filesystem under Valgrind
        add_test(
            NAME Frost_Ext_SQLite_Valgrind_${name}_file
            COMMAND ${VALGRIND} --error-exitcode=1 --leak-check=full
                    $<TARGET_FILE:frost>
                    "${SQLITE_TEST_DIR}/${script}" -- ${db_path}
        )
    endif()
endforeach()
