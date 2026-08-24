set(_GEN_CPP ${CMAKE_CURRENT_SOURCE_DIR}/generated/csv.cpp)
set_source_files_properties(${_GEN_CPP} PROPERTIES GENERATED TRUE)

add_library(frost-csv-shim
    ${_GEN_CPP}
)

add_dependencies(frost-csv-shim cargo-build_frost_rust_ext)

target_include_directories(frost-csv-shim
    PRIVATE
    ${FROST_CXXBRIDGE_INCLUDE}
)

target_link_libraries(frost-csv-shim
    PRIVATE
    frost-functions
    frost-extensions-common
    PUBLIC
    frost-rs-glue
)

if(BUILD_TESTS)
    add_test(
        NAME Frost_Ext_Csv
        COMMAND $<TARGET_FILE:frost>
                "${CMAKE_CURRENT_SOURCE_DIR}/tests/csv.frst"
    )
endif()
