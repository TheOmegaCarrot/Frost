FetchContent_Declare( Catch2
  GIT_REPOSITORY https://github.com/catchorg/Catch2.git
  GIT_TAG        v3.12.0
)

FetchContent_MakeAvailable(Catch2)

list(APPEND CMAKE_MODULE_PATH ${catch2_SOURCE_DIR}/extras)
include(Catch)

macro(make_test target file)
    add_executable(${target} ${file})
    target_link_libraries(${target} PRIVATE Catch2::Catch2WithMain)
    set_target_properties(${target} PROPERTIES
        RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/Frost_Tests
    )
    catch_discover_tests(${target}
        EXTRA_ARGS --colour-mode ansi
    ) 
endmacro()
