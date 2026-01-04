#ifndef OP_TEST_MACROS_HPP
#define OP_TEST_MACROS_HPP

#define STRINGIZE(X) #X
#define INCOMPAT(T1, T2)                                                       \
    SECTION("Incompatible: "s +                                                \
            STRINGIZE(T1) + " " + STRINGIZE(OP_CHAR) + " " + STRINGIZE(T2) )   \
    {                                                                          \
        CHECK_THROWS_WITH(                                                     \
            Value::OP_VERB(T1, T2),                                            \
            "Cannot "s + STRINGIZE(OP_VERB) + " incompatible types: " +        \
                                   STRINGIZE(T1) + " and " + STRINGIZE(T2));   \
    }

#define COMPAT(T1, T2)                                                         \
    SECTION("Compatible: "s +                                                  \
            STRINGIZE(T1) + " " + STRINGIZE(OP_CHAR) + " " + STRINGIZE(T2) )   \
    {                                                                          \
        CHECK_NOTHROW(Value::OP_VERB(T1, T2));                                 \
    }

#endif
