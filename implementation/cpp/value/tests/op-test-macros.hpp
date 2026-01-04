#ifndef OP_TEST_MACROS_HPP
#define OP_TEST_MACROS_HPP

#define OP_TEST_STRINGIZE_IMPL(X) #X
#define OP_TEST_STRINGIZE(X) OP_TEST_STRINGIZE_IMPL(X)
#define INCOMPAT(T1, T2)                                                       \
    SECTION("Incompatible: "s + OP_TEST_STRINGIZE(T1) + " " +                  \
            OP_TEST_STRINGIZE(OP_CHAR) + " " + OP_TEST_STRINGIZE(T2))          \
    {                                                                          \
        CHECK_THROWS_WITH(                                                     \
            Value::OP_VERB(T1, T2),                                            \
            "Cannot "s + OP_TEST_STRINGIZE(OP_VERB) +                          \
                " incompatible types: " + OP_TEST_STRINGIZE(T1) + " and " +    \
                OP_TEST_STRINGIZE(T2));                                        \
    }

#define COMPAT(T1, T2)                                                         \
    SECTION("Compatible: "s + OP_TEST_STRINGIZE(T1) + " " +                    \
            OP_TEST_STRINGIZE(OP_CHAR) + " " + OP_TEST_STRINGIZE(T2))          \
    {                                                                          \
        CHECK_NOTHROW(Value::OP_VERB(T1, T2));                                 \
    }

#endif
