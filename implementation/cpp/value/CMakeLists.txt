add_library( frost-value
    operators/add.cpp
    operators/subtract.cpp
    operators/multiply.cpp
    operators/divide.cpp
    operators/compare.cpp
    operators/negate.cpp
    operators/logical_ops.cpp
    to_string.cpp
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

value_test(tests/multiplication.cpp)

value_test(tests/division.cpp)

value_test(tests/eq.cpp)

value_test(tests/lt.cpp)

value_test(tests/to_string.cpp)

value_test(tests/negate.cpp)

value_test(tests/logical-ops.cpp)
