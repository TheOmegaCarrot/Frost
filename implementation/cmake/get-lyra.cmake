FetchContent_Declare( lyra
    GIT_REPOSITORY https://github.com/bfgroup/Lyra
    GIT_TAG 1.6.1
)

FetchContent_MakeAvailable(lyra)

if(NOT TARGET lyra AND TARGET lyra::lyra)
    add_library(lyra ALIAS lyra::lyra)
elseif(NOT TARGET lyra)
    add_library(lyra INTERFACE)
    target_include_directories( lyra
        INTERFACE
        ${lyra_SOURCE_DIR}/include
    )
endif()
