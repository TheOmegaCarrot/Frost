add_library( frost-unsafe
    unsafe.cpp
)

target_include_directories( frost-unsafe
    PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}
)

target_link_libraries( frost-unsafe
    PRIVATE
    frost-functions
    frost-extensions-common
)

make_test( tests/unsafe.cpp
    LIBS
    frost-unsafe
    frost-extensions-common
)

set(UNSAFE_SCRIPTS
    weaken-cell.frst
    weaken-scope.frst
)

foreach(script ${UNSAFE_SCRIPTS})
    get_filename_component(name ${script} NAME_WE)
    add_test(
        NAME Frost_Ext_Unsafe_${name}
        COMMAND $<TARGET_FILE:frost>
                "${CMAKE_CURRENT_SOURCE_DIR}/tests/${script}"
    )
endforeach()
