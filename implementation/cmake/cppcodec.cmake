add_library(cppcodec INTERFACE)

target_include_directories(cppcodec
    INTERFACE
    ${PROJECT_SOURCE_DIR}/external/cppcodec
)

add_library(cppcodec::cppcodec ALIAS cppcodec)
