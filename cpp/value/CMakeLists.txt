add_library( frost-value
    clone.cpp
    deep-equal.cpp
    explicit-conversions.cpp
    index-array.cpp
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
    frost-utils
)

set(VALUE_TEST_FILES
    basic-behavior
    coercions
    addition
    subtraction
    multiplication
    division
    modulus
    eq
    not_equal
    lt
    lte
    gt
    gte
    explicit-conversions
    negate
    logical-not
    deep-equal
    clone
    iterative-ops
)

foreach(test_file IN LISTS VALUE_TEST_FILES)
    make_test( tests/${test_file}.cpp
        LIBS
        frost-value
    )
endforeach()
