add_library( frost-value
    INTERFACE
)

target_include_directories( frost-value
    INTERFACE
    include/
)

target_link_libraries( frost-value
    INTERFACE
    frost-common
)

make_test( tests/construction.cpp
    LIBS
    frost-value
)
