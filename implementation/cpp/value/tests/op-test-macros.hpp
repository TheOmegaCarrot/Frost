#ifndef OP_TEST_MACROS_HPP
#define OP_TEST_MACROS_HPP

#include <boost/preprocessor/stringize.hpp>

#define INCOMPAT(T1, T2)                                                       \
    SECTION("Incompatible: "s                                                  \
            + BOOST_PP_STRINGIZE(T1)                                               \
                + " "                                                          \
                + BOOST_PP_STRINGIZE(OP_CHAR) + " " + BOOST_PP_STRINGIZE(T2))  \
    {                                                                          \
        CHECK_THROWS_WITH(                                                     \
            Value::OP_METHOD(T1, T2),                                          \
            "Cannot "s                                                         \
                + BOOST_PP_STRINGIZE(OP_VERB)                                           \
                    + " incompatible types: "                                  \
                    + BOOST_PP_STRINGIZE(T1)                                       \
                        + " "                                                  \
                        + BOOST_PP_STRINGIZE(OP_CHAR) + " " + BOOST_PP_STRINGIZE(T2));          \
    }

#define COMPAT(T1, T2)                                                         \
    SECTION("Compatible: "s                                                    \
            + BOOST_PP_STRINGIZE(T1)                                               \
                + " "                                                          \
                + BOOST_PP_STRINGIZE(OP_CHAR) + " " + BOOST_PP_STRINGIZE(T2))  \
    {                                                                          \
        CHECK_NOTHROW(Value::OP_METHOD(T1, T2));                               \
    }

#endif
