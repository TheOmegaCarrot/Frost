if(BUILD_TESTS)
    FetchContent_Declare( Catch2
      GIT_REPOSITORY https://github.com/catchorg/Catch2.git
      GIT_TAG        v3.12.0
    )

    FetchContent_MakeAvailable(Catch2)

    list(APPEND CMAKE_MODULE_PATH ${catch2_SOURCE_DIR}/extras)
    include(Catch)

    add_library( frost-testing INTERFACE )
    target_link_libraries( frost-testing
        INTERFACE
        Catch2::Catch2WithMain
    )
    target_include_directories( frost-testing
        INTERFACE
        ${CMAKE_CURRENT_SOURCE_DIR}/external/trompeloeil/include
        ${CMAKE_CURRENT_SOURCE_DIR}/cpp/test-helpers/include
    )
endif()

macro(make_test file)
    if(BUILD_TESTS)
        set(multi_value_args LIBS)
        cmake_parse_arguments(MAKE_TEST "" "" "${multi_value_args}" ${ARGN})

        if(IS_ABSOLUTE "${file}")
            set(test_source "${file}")
        else()
            set(test_source "${CMAKE_CURRENT_SOURCE_DIR}/${file}")
        endif()
        set_property(GLOBAL APPEND PROPERTY FROST_UNIT_TEST_SOURCES "${test_source}")

        get_filename_component(target ${file} NAME_WE)
        add_executable(${target} ${file})

        target_link_libraries( ${target}
            PRIVATE
            frost-testing
            ${MAKE_TEST_LIBS}
        )

        set_target_properties( ${target} PROPERTIES
            RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/Frost_Tests
        )

        catch_discover_tests( ${target}
            EXTRA_ARGS --colour-mode ansi
        ) 
    endif()
endmacro()
