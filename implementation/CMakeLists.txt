cmake_minimum_required(VERSION 3.31.6)

project(
    frost
    VERSION 1.0.0
    LANGUAGES CXX
)

set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

if(NOT DEFINED CMAKE_BUILD_TYPE)
    message(FATAL_ERROR "Build type not specified!")
endif()

message(STATUS "Build type: ${CMAKE_BUILD_TYPE}")

option(BUILD_TESTS "Build tests" YES)

list(APPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_SOURCE_DIR}/cmake")

include(CTest)
include(FetchContent)
include(get-lexy)
include(get-fmt)
include(get-replxx)
include(lyra)
include(cppcodec)

include(get-test-libs)

find_package(Boost REQUIRED COMPONENTS regex json headers)

if(${CMAKE_BUILD_TYPE} STREQUAL "Release")
    set(CMAKE_INTERPROCEDURAL_OPTIMIZATION YES)
endif()

add_library(frost-common INTERFACE)

target_compile_features( frost-common
    INTERFACE
    cxx_std_26
)

target_compile_options( frost-common
    INTERFACE
    -Wall
    -Wextra
    -Wpedantic
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
add_subdirectory(integration-tests/)
