add_library(lyra INTERFACE)

target_include_directories(lyra
    INTERFACE
    ${PROJECT_SOURCE_DIR}/external/lyra/include
)

add_library(lyra::lyra ALIAS lyra)
