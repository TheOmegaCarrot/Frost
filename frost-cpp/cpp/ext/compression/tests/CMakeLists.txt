make_test( compression.cpp
    LIBS
    frost-compression
    frost-extensions-common
)

if(BUILD_TESTS)
    target_compile_definitions(compression PRIVATE ${COMPRESSION_DEFS})
endif()
