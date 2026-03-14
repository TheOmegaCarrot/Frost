cmake_minimum_required(VERSION 3.31.6)

project(
    frost
    VERSION 0.0.2
    LANGUAGES CXX
)

set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

if(NOT DEFINED CMAKE_BUILD_TYPE)
    message(FATAL_ERROR "Build type not specified!")
endif()

message(STATUS "Build type: ${CMAKE_BUILD_TYPE}")

option(BUILD_TESTS "Build tests" YES)
option(WITH_HTTP "Enable HTTP support" YES)

list(APPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_SOURCE_DIR}/cmake")

include(CTest)
include(FetchContent)
include(get-lexy)
include(get-fmt)
include(get-replxx)
include(lyra)
include(cppcodec)
include(clang-tidy-targets)

include(get-test-libs)

find_package(Boost REQUIRED COMPONENTS regex json headers)

set(CMAKE_INTERPROCEDURAL_OPTIMIZATION_RELEASE YES)
set(CMAKE_INTERPROCEDURAL_OPTIMIZATION_RELWITHDEBINFO YES)

add_library(frost-common INTERFACE)

if(NOT MSVC)
    target_link_options( frost-common
        INTERFACE
        -fuse-ld=mold
    )
endif()

target_compile_features( frost-common
    INTERFACE
    cxx_std_26
)

target_compile_options( frost-common
    INTERFACE
    -Wall
    -Wextra
    -Wpedantic
    "$<$<CONFIG:RelWithDebInfo>:-O3;-g;-fno-omit-frame-pointer>"
)

target_link_libraries( frost-common
    INTERFACE
    fmt::fmt
)

target_compile_definitions( frost-common
    INTERFACE
    FROST_VERSION="${PROJECT_VERSION}"
)

add_subdirectory(cpp/)

set(FROST_INTERPRETER_TARGETS
    frost
    frost-ast
    frost-ext
    frost-functions
    frost-http
    frost-meta
    frost-parser
    frost-prelude
    frost-execution-context
    frost-utils
    frost-value
)
frost_collect_target_sources(
    FROST_INTERPRETER_SOURCES
    TARGETS ${FROST_INTERPRETER_TARGETS}
    EXCLUDE_TESTS
)
frost_create_clang_tidy_target(
    check
    GROUP_LABEL interpreter
    SOURCES ${FROST_INTERPRETER_SOURCES}
)

get_property(FROST_UNIT_TEST_SOURCES GLOBAL PROPERTY FROST_UNIT_TEST_SOURCES)
frost_create_clang_tidy_target(
    check-tests
    GROUP_LABEL tests
    SOURCES ${FROST_UNIT_TEST_SOURCES}
)

if(CMAKE_BUILD_TYPE STREQUAL "RelWithDebInfo")
    add_subdirectory(frost-scripts/performance-test/)
endif()
add_subdirectory(integration-tests/)
