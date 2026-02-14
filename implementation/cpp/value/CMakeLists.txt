add_library( frost-value
    clone.cpp
    deep-equal.cpp
    explicit-conversions.cpp
    internal-map-compare.cpp
    iterative-ops.cpp
    operators/add.cpp
    operators/subtract.cpp
    operators/multiply.cpp
    operators/divide.cpp
    operators/compare.cpp
    operators/negate.cpp
    operators/modulus.cpp
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

value_test(tests/modulus.cpp)

value_test(tests/eq.cpp)

value_test(tests/not_equal.cpp)

value_test(tests/lt.cpp)

value_test(tests/lte.cpp)

value_test(tests/gt.cpp)

value_test(tests/gte.cpp)

value_test(tests/explicit-conversions.cpp)

value_test(tests/negate.cpp)

value_test(tests/logical-not.cpp)

value_test(tests/deep-equal.cpp)

value_test(tests/clone.cpp)

value_test(tests/iterative-ops.cpp)
