CPMAddPackage(
    NAME tomlplusplus
    GITHUB_REPOSITORY marzer/tomlplusplus
    GIT_TAG v3.4.0
)

add_library( frost-toml
    toml.cpp
)

target_include_directories( frost-toml
    PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}
)

target_link_libraries( frost-toml
    PRIVATE
    frost-functions
    frost-extensions-common
    tomlplusplus::tomlplusplus
)

make_test( tests/toml.cpp
    LIBS
    frost-toml
    frost-extensions-common
    tomlplusplus::tomlplusplus
)
