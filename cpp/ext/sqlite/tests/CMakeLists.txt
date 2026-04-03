set(SQLITE_TEST_DIR "${CMAKE_CURRENT_SOURCE_DIR}")

set(SQLITE_TESTS
    connection.frst
    exec.frst
    query.frst
    each.frst
    collect.frst
    script.frst
    transaction.frst
    named-bindings.frst
    callback-nesting.frst
    fts5.frst
    math-functions.frst
    csv-extension.frst
    fileio.frst
    sqlar-compress.frst
    generate-series.frst
    regexp.frst
    sha3.frst
    uuid.frst
    foreign-keys.frst
    soundex.frst
    dbstat.frst
    percentile.frst
)

find_program(VALGRIND valgrind)

foreach(script ${SQLITE_TESTS})
    get_filename_component(name ${script} NAME_WE)

    # In-memory
    add_test(
        NAME Frost_Ext_SQLite_${name}_memory
        COMMAND $<TARGET_FILE:frost>
                "${SQLITE_TEST_DIR}/${script}" -- :memory:
    )

    # Filesystem (each variant gets its own db file to avoid races under parallel CTest)
    add_test(
        NAME Frost_Ext_SQLite_${name}_file
        COMMAND $<TARGET_FILE:frost>
                "${SQLITE_TEST_DIR}/${script}" -- "${CMAKE_CURRENT_BINARY_DIR}/${name}.db"
    )

    if(VALGRIND)
        # In-memory under Valgrind
        add_test(
            NAME Frost_Ext_SQLite_Valgrind_${name}_memory
            COMMAND ${VALGRIND} --error-exitcode=1 --leak-check=full
                    $<TARGET_FILE:frost>
                    "${SQLITE_TEST_DIR}/${script}" -- :memory:
        )

        # Filesystem under Valgrind (separate db file from the non-Valgrind variant)
        add_test(
            NAME Frost_Ext_SQLite_Valgrind_${name}_file
            COMMAND ${VALGRIND} --error-exitcode=1 --leak-check=full
                    $<TARGET_FILE:frost>
                    "${SQLITE_TEST_DIR}/${script}" -- "${CMAKE_CURRENT_BINARY_DIR}/${name}-valgrind.db"
        )
    endif()
endforeach()
