add_library( frost-value
    operators/add.cpp
    operators/minus.cpp
)

target_include_directories( frost-value
    PUBLIC
    include/
)

target_link_libraries( frost-value
    PUBLIC
    frost-common
)

macro(value_test file)
    make_test( ${file}
        LIBS
        frost-value
    )
endmacro()

value_test(tests/basic-behavior.cpp)

value_test(tests/coercions.cpp)

value_test(tests/addition.cpp)

value_test(tests/subtraction.cpp)
