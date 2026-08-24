CPMAddPackage(
    NAME cppcodec
    GITHUB_REPOSITORY tplgy/cppcodec
    GIT_TAG v0.2
    DOWNLOAD_ONLY YES
)

add_library(cppcodec INTERFACE)
target_include_directories(cppcodec INTERFACE ${cppcodec_SOURCE_DIR})
add_library(cppcodec::cppcodec ALIAS cppcodec)
