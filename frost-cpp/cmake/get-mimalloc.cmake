if(CMAKE_BUILD_TYPE STREQUAL "Release")
    set(FROST_MI_VALGRIND OFF)
else()
    find_program(VALGRIND_EXECUTABLE valgrind)
    if(VALGRIND_EXECUTABLE)
        set(FROST_MI_VALGRIND ON)
    else()
        set(FROST_MI_VALGRIND OFF)
    endif()
endif()

CPMAddPackage(
    NAME mimalloc
    GITHUB_REPOSITORY microsoft/mimalloc
    GIT_TAG v2.2.6
    OPTIONS
        "MI_BUILD_SHARED OFF"
        "MI_BUILD_TESTS OFF"
        "MI_BUILD_OBJECT OFF"
        "MI_VALGRIND ${FROST_MI_VALGRIND}"
)
