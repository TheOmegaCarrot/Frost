CPMAddPackage(
    NAME msgpack-cxx
    GITHUB_REPOSITORY msgpack/msgpack-c
    GIT_TAG cpp-7.0.0
    OPTIONS
        "MSGPACK_USE_BOOST OFF"
        "MSGPACK_BUILD_TESTS OFF"
        "MSGPACK_BUILD_EXAMPLES OFF"
)

add_library( frost-msgpack
    msgpack.cpp
)

target_include_directories( frost-msgpack
    PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}
)

target_link_libraries( frost-msgpack
    PRIVATE
    frost-functions
    frost-extensions-common
    msgpack-cxx
)

make_test( tests/msgpack.cpp
    LIBS
    frost-msgpack
    frost-extensions-common
    msgpack-cxx
)

set(MSGPACK_SCRIPTS
    encode.frst
)

foreach(script ${MSGPACK_SCRIPTS})
    get_filename_component(name ${script} NAME_WE)
    add_test(
        NAME Frost_Ext_Msgpack_${name}
        COMMAND $<TARGET_FILE:frost>
                "${CMAKE_CURRENT_SOURCE_DIR}/tests/${script}"
    )
endforeach()
