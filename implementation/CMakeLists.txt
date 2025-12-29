cmake_minimum_required(VERSION 4.2.1)

project(
    frost
    VERSION 0.0.1
    LANGUAGES CXX
)

set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

if(NOT DEFINED CMAKE_BUILD_TYPE)
    message(FATAL_ERROR "Build type not specified!")
endif()

message(STATUS "Build type: ${CMAKE_BUILD_TYPE}")

list(APPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_SOURCE_DIR}/cmake")

include(FetchContent)
include(get-lexy)

add_library(common INTERFACE)
target_compile_features(common INTERFACE cxx_std_26)
target_compile_options(common INTERFACE
    -Wall
    -Wextra
)

add_subdirectory(cpp/)
