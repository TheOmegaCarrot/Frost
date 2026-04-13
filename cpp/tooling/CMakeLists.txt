
option(BUILD_ASTVIZ "Build AST visualizer tool" NO)

if(BUILD_ASTVIZ)
    add_subdirectory(astviz/)
endif()
