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

list(append CMAKE_MODULE_PATH "${CMAKE_CURRENT_SOURCE_DIR}/cmake")

include(FetchContent)
include(get-lexy)

add_subdirectory(cpp/)
