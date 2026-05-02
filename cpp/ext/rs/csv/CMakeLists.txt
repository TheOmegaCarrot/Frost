corrosion_import_crate(MANIFEST_PATH ${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml)

set(_GEN_CPP ${CMAKE_CURRENT_SOURCE_DIR}/generated/csv.cpp)
set_source_files_properties(${_GEN_CPP} PROPERTIES GENERATED TRUE)

add_library(frost-csv-shim
    ${_GEN_CPP}
)

add_dependencies(frost-csv-shim cargo-build_frost_csv)

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
    ${CMAKE_CURRENT_BINARY_DIR}/libfrost_csv.a
)
