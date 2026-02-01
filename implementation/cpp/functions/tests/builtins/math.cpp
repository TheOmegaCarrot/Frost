#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_exception.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <cmath>
#include <limits>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <frost/builtin.hpp>

using namespace frst;

using namespace std::literals;

using namespace Catch::Matchers;

TEST_CASE("Builtin math")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    Symbol_Table table;
    inject_builtins(table);

    auto get_fn = [&](const std::string& name) {
        auto val = table.lookup(name);
        REQUIRE(val->is<Function>());
        return val->get<Function>().value();
    };

    const std::vector<std::string> unary_names{
        "sqrt", "cbrt",  "sin",   "cos",   "tan",   "asin",  "acos",
        "atan", "sinh",  "cosh",  "tanh",  "asinh", "acosh", "atanh",
        "log",  "log1p", "log2",  "log10", "ceil",  "floor", "trunc",
        "exp",  "exp2",  "expm1", "abs",   "round",
    };
    const std::vector<std::string> binary_names{"pow", "min", "max", "atan2"};
    const std::vector<std::string> variadic_names{"hypot"};
    const std::vector<std::string> int_binary_names{"mod"};

    SECTION("Injected")
    {
        for (const auto& name : unary_names)
        {
            auto val = table.lookup(name);
            REQUIRE(val->is<Function>());
        }
        for (const auto& name : binary_names)
        {
            auto val = table.lookup(name);
            REQUIRE(val->is<Function>());
        }
        for (const auto& name : variadic_names)
        {
            auto val = table.lookup(name);
            REQUIRE(val->is<Function>());
        }
        for (const auto& name : int_binary_names)
        {
            auto val = table.lookup(name);
            REQUIRE(val->is<Function>());
        }
    }

    SECTION("Arity")
    {
        struct Arity_Case
        {
            std::string name;
            std::size_t min;
            std::size_t max;
        };

        const std::vector<Arity_Case> cases{
            {.name = "sqrt", .min = 1, .max = 1},
            {.name = "cbrt", .min = 1, .max = 1},
            {.name = "sin", .min = 1, .max = 1},
            {.name = "cos", .min = 1, .max = 1},
            {.name = "tan", .min = 1, .max = 1},
            {.name = "asin", .min = 1, .max = 1},
            {.name = "acos", .min = 1, .max = 1},
            {.name = "atan", .min = 1, .max = 1},
            {.name = "sinh", .min = 1, .max = 1},
            {.name = "cosh", .min = 1, .max = 1},
            {.name = "tanh", .min = 1, .max = 1},
            {.name = "asinh", .min = 1, .max = 1},
            {.name = "acosh", .min = 1, .max = 1},
            {.name = "atanh", .min = 1, .max = 1},
            {.name = "log", .min = 1, .max = 1},
            {.name = "log1p", .min = 1, .max = 1},
            {.name = "log2", .min = 1, .max = 1},
            {.name = "log10", .min = 1, .max = 1},
            {.name = "ceil", .min = 1, .max = 1},
            {.name = "floor", .min = 1, .max = 1},
            {.name = "trunc", .min = 1, .max = 1},
            {.name = "exp", .min = 1, .max = 1},
            {.name = "exp2", .min = 1, .max = 1},
            {.name = "expm1", .min = 1, .max = 1},
            {.name = "abs", .min = 1, .max = 1},
            {.name = "round", .min = 1, .max = 1},
            {.name = "pow", .min = 2, .max = 2},
            {.name = "min", .min = 2, .max = 2},
            {.name = "max", .min = 2, .max = 2},
            {.name = "atan2", .min = 2, .max = 2},
            {.name = "hypot", .min = 2, .max = 3},
            {.name = "mod", .min = 2, .max = 2},
        };

        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto c = Value::create(3_f);
        auto d = Value::create(4_f);

        for (const auto& test : cases)
        {
            DYNAMIC_SECTION("Arity " << test.name)
            {
                auto fn = get_fn(test.name);

                if (test.min == 1)
                {
                    CHECK_THROWS_MATCHES(
                        fn->call({}), Frost_User_Error,
                        MessageMatches(
                            ContainsSubstring("insufficient arguments")
                            && ContainsSubstring("Called with 0")
                            && ContainsSubstring("requires at least 1")));
                }
                else if (test.min == 2)
                {
                    CHECK_THROWS_MATCHES(
                        fn->call({a}), Frost_User_Error,
                        MessageMatches(
                            ContainsSubstring("insufficient arguments")
                            && ContainsSubstring("Called with 1")
                            && ContainsSubstring("requires at least 2")));
                }

                if (test.max == 1)
                {
                    CHECK_THROWS_MATCHES(
                        fn->call({a, b}), Frost_User_Error,
                        MessageMatches(ContainsSubstring("too many arguments")
                                       && ContainsSubstring("Called with 2")
                                       && ContainsSubstring("no more than 1")));
                }
                else if (test.max == 2)
                {
                    CHECK_THROWS_MATCHES(
                        fn->call({a, b, c}), Frost_User_Error,
                        MessageMatches(ContainsSubstring("too many arguments")
                                       && ContainsSubstring("Called with 3")
                                       && ContainsSubstring("no more than 2")));
                }
                else if (test.max == 3)
                {
                    CHECK_THROWS_MATCHES(
                        fn->call({a, b, c, d}), Frost_User_Error,
                        MessageMatches(ContainsSubstring("too many arguments")
                                       && ContainsSubstring("Called with 4")
                                       && ContainsSubstring("no more than 3")));
                }
            }
        }
    }

    SECTION("Type errors")
    {
        auto bad = Value::create("nope"s);
        auto good = Value::create(1_f);

        for (const auto& name : unary_names)
        {
            auto fn = get_fn(name);
            CHECK_THROWS_MATCHES(
                fn->call({bad}), Frost_User_Error,
                MessageMatches(ContainsSubstring("Function " + name)
                               && ContainsSubstring("got String")));
        }

        for (const auto& name : binary_names)
        {
            auto fn = get_fn(name);
            CHECK_THROWS_MATCHES(
                fn->call({bad, good}), Frost_User_Error,
                MessageMatches(ContainsSubstring("Function " + name)
                               && ContainsSubstring("got String")));
            CHECK_THROWS_MATCHES(
                fn->call({good, bad}), Frost_User_Error,
                MessageMatches(ContainsSubstring("Function " + name)
                               && ContainsSubstring("got String")));
        }

        for (const auto& name : variadic_names)
        {
            auto fn = get_fn(name);
            CHECK_THROWS_MATCHES(
                fn->call({bad, good}), Frost_User_Error,
                MessageMatches(ContainsSubstring("Function " + name)
                               && ContainsSubstring("got String")));
            CHECK_THROWS_MATCHES(
                fn->call({good, bad}), Frost_User_Error,
                MessageMatches(ContainsSubstring("Function " + name)
                               && ContainsSubstring("got String")));
            CHECK_THROWS_MATCHES(
                fn->call({good, good, bad}), Frost_User_Error,
                MessageMatches(ContainsSubstring("Function " + name)
                               && ContainsSubstring("got String")));
        }

        auto mod_fn = get_fn("mod");
        CHECK_THROWS_MATCHES(
            mod_fn->call({bad, good}), Frost_User_Error,
            MessageMatches(ContainsSubstring("Function mod")
                           && ContainsSubstring("got String")));
        CHECK_THROWS_MATCHES(
            mod_fn->call({good, bad}), Frost_User_Error,
            MessageMatches(ContainsSubstring("Function mod")
                           && ContainsSubstring("got String")));
        CHECK_THROWS_MATCHES(
            mod_fn->call({Value::create(1.5), Value::create(2_f)}),
            Frost_User_Error,
            MessageMatches(ContainsSubstring("Function mod")
                           && ContainsSubstring("got Float")));
    }

    SECTION("Unary math functions")
    {
        using Unary_Fn = double (*)(double);
        struct Unary_Case
        {
            std::string name;
            Unary_Fn fn;
            long long int_in;
            double float_in;
        };

        const std::vector<Unary_Case> cases{
            {
                .name = "sqrt",
                .fn = std::sqrt,
                .int_in = 4,
                .float_in = 2.25,
            },
            {
                .name = "cbrt",
                .fn = std::cbrt,
                .int_in = 8,
                .float_in = 0.125,
            },
            {
                .name = "sin",
                .fn = std::sin,
                .int_in = 0,
                .float_in = 0.5,
            },
            {
                .name = "cos",
                .fn = std::cos,
                .int_in = 0,
                .float_in = 1.0,
            },
            {
                .name = "tan",
                .fn = std::tan,
                .int_in = 0,
                .float_in = 0.5,
            },
            {
                .name = "asin",
                .fn = std::asin,
                .int_in = 0,
                .float_in = 0.5,
            },
            {
                .name = "acos",
                .fn = std::acos,
                .int_in = 1,
                .float_in = 0.0,
            },
            {
                .name = "atan",
                .fn = std::atan,
                .int_in = 0,
                .float_in = 1.0,
            },
            {
                .name = "sinh",
                .fn = std::sinh,
                .int_in = 0,
                .float_in = 1.0,
            },
            {
                .name = "cosh",
                .fn = std::cosh,
                .int_in = 0,
                .float_in = 1.0,
            },
            {
                .name = "tanh",
                .fn = std::tanh,
                .int_in = 0,
                .float_in = 1.0,
            },
            {
                .name = "asinh",
                .fn = std::asinh,
                .int_in = 0,
                .float_in = 1.0,
            },
            {
                .name = "acosh",
                .fn = std::acosh,
                .int_in = 1,
                .float_in = 2.0,
            },
            {
                .name = "atanh",
                .fn = std::atanh,
                .int_in = 0,
                .float_in = 0.5,
            },
            {
                .name = "log",
                .fn = std::log,
                .int_in = 1,
                .float_in = 2.0,
            },
            {
                .name = "log1p",
                .fn = std::log1p,
                .int_in = 0,
                .float_in = 0.5,
            },
            {
                .name = "log2",
                .fn = std::log2,
                .int_in = 1,
                .float_in = 2.0,
            },
            {
                .name = "log10",
                .fn = std::log10,
                .int_in = 1,
                .float_in = 10.0,
            },
            {
                .name = "ceil",
                .fn = std::ceil,
                .int_in = 2,
                .float_in = 2.3,
            },
            {
                .name = "floor",
                .fn = std::floor,
                .int_in = 2,
                .float_in = 2.7,
            },
            {
                .name = "trunc",
                .fn = std::trunc,
                .int_in = 2,
                .float_in = -2.7,
            },
            {
                .name = "exp",
                .fn = std::exp,
                .int_in = 0,
                .float_in = 1.0,
            },
            {
                .name = "exp2",
                .fn = std::exp2,
                .int_in = 0,
                .float_in = 3.0,
            },
            {
                .name = "expm1",
                .fn = std::expm1,
                .int_in = 0,
                .float_in = 0.5,
            },
        };

        for (const auto& test : cases)
        {
            DYNAMIC_SECTION("Unary " << test.name)
            {
                auto fn = get_fn(test.name);

                auto res_int = fn->call({Value::create(Int{test.int_in})});
                REQUIRE(res_int->is<Float>());
                CHECK(res_int->get<Float>().value()
                      == Catch::Approx(
                          test.fn(static_cast<double>(test.int_in))));

                auto res_float = fn->call(
                    {Value::create(static_cast<double>(test.float_in))});
                REQUIRE(res_float->is<Float>());
                CHECK(res_float->get<Float>().value()
                      == Catch::Approx(
                          test.fn(static_cast<double>(test.float_in))));
            }
        }
    }

    SECTION("Binary math functions")
    {
        using Binary_Fn = double (*)(double, double);
        struct Binary_Case
        {
            std::string name;
            Binary_Fn fn;
            long long a_int;
            long long b_int;
            double a_float;
            double b_float;
        };

        const std::vector<Binary_Case> cases{
            {
                .name = "pow",
                .fn = std::pow,
                .a_int = 2,
                .b_int = 3,
                .a_float = 2.5,
                .b_float = 3.0,
            },
            {
                .name = "min",
                .fn = std::fmin,
                .a_int = 2,
                .b_int = 3,
                .a_float = 2.5,
                .b_float = -1.0,
            },
            {
                .name = "max",
                .fn = std::fmax,
                .a_int = 2,
                .b_int = 3,
                .a_float = 2.5,
                .b_float = -1.0,
            },
            {
                .name = "atan2",
                .fn = std::atan2,
                .a_int = 1,
                .b_int = 1,
                .a_float = 1.5,
                .b_float = -0.5,
            },
        };

        for (const auto& test : cases)
        {
            DYNAMIC_SECTION("Binary " << test.name)
            {
                auto fn = get_fn(test.name);

                auto res_int = fn->call({Value::create(Int{test.a_int}),
                                         Value::create(Int{test.b_int})});
                REQUIRE(res_int->is<Float>());
                CHECK(
                    res_int->get<Float>().value()
                    == Catch::Approx(test.fn(static_cast<double>(test.a_int),
                                             static_cast<double>(test.b_int))));

                auto res_float = fn->call(
                    {Value::create(static_cast<double>(test.a_float)),
                     Value::create(static_cast<double>(test.b_float))});
                REQUIRE(res_float->is<Float>());
                CHECK(res_float->get<Float>().value()
                      == Catch::Approx(
                          test.fn(static_cast<double>(test.a_float),
                                  static_cast<double>(test.b_float))));
            }
        }
    }

    SECTION("abs")
    {
        auto fn = get_fn("abs");
        auto i = fn->call({Value::create(-5_f)});
        REQUIRE(i->is<Int>());
        CHECK(i->get<Int>() == 5_f);

        auto f = fn->call({Value::create(-2.5)});
        REQUIRE(f->is<Float>());
        CHECK(f->get<Float>().value() == Catch::Approx(2.5));

        CHECK_THROWS_MATCHES(
            fn->call({Value::create(std::numeric_limits<Int>::min())}),
            Frost_User_Error,
            MessageMatches(ContainsSubstring("Function abs")
                           && ContainsSubstring("minimum Int")));
    }

    SECTION("round")
    {
        auto fn = get_fn("round");
        auto i = fn->call({Value::create(7_f)});
        REQUIRE(i->is<Int>());
        CHECK(i->get<Int>() == 7_f);

        auto pos = fn->call({Value::create(2.5)});
        REQUIRE(pos->is<Int>());
        CHECK(pos->get<Int>() == 3_f);

        auto neg = fn->call({Value::create(-2.5)});
        REQUIRE(neg->is<Int>());
        CHECK(neg->get<Int>() == -3_f);

        CHECK_THROWS_MATCHES(
            fn->call({Value::create(std::numeric_limits<Float>::max())}),
            Frost_User_Error,
            MessageMatches(ContainsSubstring("out of range of Int")));
    }

    SECTION("hypot")
    {
        auto fn = get_fn("hypot");

        auto res_int = fn->call({Value::create(3_f), Value::create(4_f)});
        REQUIRE(res_int->is<Float>());
        CHECK(res_int->get<Float>().value() == Catch::Approx(5.0));

        auto res_float = fn->call({Value::create(6.0), Value::create(8.0)});
        REQUIRE(res_float->is<Float>());
        CHECK(res_float->get<Float>().value() == Catch::Approx(10.0));

        auto res_three = fn->call(
            {Value::create(1_f), Value::create(2_f), Value::create(2_f)});
        REQUIRE(res_three->is<Float>());
        CHECK(res_three->get<Float>().value()
              == Catch::Approx(std::hypot(1.0, 2.0, 2.0)));
    }

    SECTION("mod")
    {
        auto fn = get_fn("mod");

        auto pos = fn->call({Value::create(5_f), Value::create(2_f)});
        REQUIRE(pos->is<Int>());
        CHECK(pos->get<Int>() == 1_f);

        auto neg_lhs = fn->call({Value::create(-5_f), Value::create(2_f)});
        REQUIRE(neg_lhs->is<Int>());
        CHECK(neg_lhs->get<Int>() == -1_f);

        auto neg_rhs = fn->call({Value::create(5_f), Value::create(-2_f)});
        REQUIRE(neg_rhs->is<Int>());
        CHECK(neg_rhs->get<Int>() == 1_f);

        auto neg_both = fn->call({Value::create(-5_f), Value::create(-2_f)});
        REQUIRE(neg_both->is<Int>());
        CHECK(neg_both->get<Int>() == -1_f);

        CHECK_THROWS_MATCHES(
            fn->call({Value::create(1_f), Value::create(0_f)}),
            Frost_User_Error,
            MessageMatches(ContainsSubstring("Cannot modulus by 0")));

        CHECK_THROWS_MATCHES(
            fn->call({Value::create(std::numeric_limits<Int>::min()),
                      Value::create(-1_f)}),
            Frost_User_Error,
            MessageMatches(ContainsSubstring("Function mod")
                           && ContainsSubstring("minimum Int")));
    }
}
