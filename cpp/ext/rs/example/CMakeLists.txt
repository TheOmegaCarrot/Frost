set(_GEN_CPP ${CMAKE_CURRENT_SOURCE_DIR}/generated/example.cpp)
set_source_files_properties(${_GEN_CPP} PROPERTIES GENERATED TRUE)

add_library(frost-example-shim
    ${_GEN_CPP}
)

add_dependencies(frost-example-shim cargo-build_frost_rust_ext)

target_include_directories(frost-example-shim
    PRIVATE
    ${FROST_CXXBRIDGE_INCLUDE}
)

target_link_libraries(frost-example-shim
    PRIVATE
    frost-functions
    frost-extensions-common
    PUBLIC
    frost-rs-glue
)

if(BUILD_TESTS)
    add_test(
        NAME Frost_Ext_Example_rust_ffi
        COMMAND $<TARGET_FILE:frost>
                "${CMAKE_CURRENT_SOURCE_DIR}/tests/example.frst"
    )

    add_test(
        NAME Frost_Ext_Example_bridge_error_type
        COMMAND $<TARGET_FILE:frost>
                "${CMAKE_CURRENT_SOURCE_DIR}/tests/bridge-error-type.frst"
    )
    set_tests_properties(Frost_Ext_Example_bridge_error_type PROPERTIES WILL_FAIL TRUE)

    add_test(
        NAME Frost_Ext_Example_bridge_backtrace
        COMMAND $<TARGET_FILE:frost> --enable-backtrace
                "${CMAKE_CURRENT_SOURCE_DIR}/tests/bridge-backtrace.frst"
    )

    add_test(
        NAME Frost_Ext_Example_bridge_backtrace_rust_error
        COMMAND $<TARGET_FILE:frost> --enable-backtrace
                "${CMAKE_CURRENT_SOURCE_DIR}/tests/bridge-backtrace-rust-error.frst"
    )
endif()
