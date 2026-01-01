add_library( frost-value
    operators/add.cpp
)

target_include_directories( frost-value
    PUBLIC
    include/
)

target_link_libraries( frost-value
    PUBLIC
    frost-common
)

make_test( tests/basic-behavior.cpp
    LIBS
    frost-value
)

make_test( tests/coercions.cpp
    LIBS
    frost-value
)
