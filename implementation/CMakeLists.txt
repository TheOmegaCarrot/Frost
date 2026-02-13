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
option(WITH_HTTP "Enable HTTP support" YES)

list(APPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_SOURCE_DIR}/cmake")

include(CTest)
include(FetchContent)
include(get-lexy)
include(get-fmt)
include(get-replxx)
include(lyra)
include(cppcodec)

include(get-test-libs)

if(WITH_HTTP)
    find_package(Boost REQUIRED COMPONENTS regex json headers url)
    find_package(OpenSSL REQUIRED)
else()
    find_package(Boost REQUIRED COMPONENTS regex json headers)
endif()

set(CMAKE_INTERPROCEDURAL_OPTIMIZATION_RELEASE YES)

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
)

target_link_libraries( frost-common
    INTERFACE
    fmt::fmt
)

target_compile_definitions( frost-common
    INTERFACE
    FROST_VERSION="${PROJECT_VERSION}"
)

if(WITH_HTTP)
    message("Enabling HTTP")
    target_compile_definitions( frost-common
    INTERFACE
    FROST_ENABLE_HTTP
    )
endif()

add_subdirectory(cpp/)
add_subdirectory(integration-tests/)
