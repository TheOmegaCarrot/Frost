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
)

set(SQLITE_FEATURE_CHECKS
    feature-checks/csv-extension.frst
    feature-checks/dbstat.frst
    feature-checks/decimal.frst
    feature-checks/fileio.frst
    feature-checks/foreign-keys.frst
    feature-checks/fts5.frst
    feature-checks/generate-series.frst
    feature-checks/math-functions.frst
    feature-checks/percentile.frst
    feature-checks/regexp.frst
    feature-checks/sha3.frst
    feature-checks/soundex.frst
    feature-checks/sqlar-compress.frst
    feature-checks/uuid.frst
    feature-checks/zipfile.frst
    feature-checks/create-function.frst
)

find_program(VALGRIND valgrind)

foreach(script ${SQLITE_TESTS} ${SQLITE_FEATURE_CHECKS})
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
