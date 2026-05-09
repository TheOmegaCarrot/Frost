set(_GEN_CPP ${CMAKE_CURRENT_SOURCE_DIR}/generated/ini.cpp)
set_source_files_properties(${_GEN_CPP} PROPERTIES GENERATED TRUE)

add_library(frost-ini-shim
    ${_GEN_CPP}
)

add_dependencies(frost-ini-shim cargo-build_frost_rust_ext)

target_include_directories(frost-ini-shim
    PRIVATE
    ${FROST_CXXBRIDGE_INCLUDE}
)

target_link_libraries(frost-ini-shim
    PRIVATE
    frost-functions
    frost-extensions-common
    PUBLIC
    frost-rs-glue
)

if(BUILD_TESTS)
    add_test(
        NAME Frost_Ext_Ini
        COMMAND $<TARGET_FILE:frost>
                "${CMAKE_CURRENT_SOURCE_DIR}/tests/ini.frst"
    )
endif()
