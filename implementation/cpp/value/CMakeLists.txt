add_library( frost-value
    operators/add.cpp
    operators/subtract.cpp
    operators/multiply.cpp
    operators/divide.cpp
    operators/compare.cpp
    operators/negate.cpp
    explicit-conversions.cpp
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

# AI-generated test entry by Codex (GPT-5).
value_test(tests/not_equal.cpp)

value_test(tests/lt.cpp)

# AI-generated test entries by Codex (GPT-5).
value_test(tests/lte.cpp)

# AI-generated test entries by Codex (GPT-5).
value_test(tests/gt.cpp)

# AI-generated test entries by Codex (GPT-5).
value_test(tests/gte.cpp)

value_test(tests/explicit-conversions.cpp)

value_test(tests/negate.cpp)

value_test(tests/logical-not.cpp)
