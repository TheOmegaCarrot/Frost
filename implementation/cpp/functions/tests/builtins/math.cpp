#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_exception.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <cmath>

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
        "sqrt", "cbrt",  "sin",  "cos",   "tan",   "asin",  "acos",
        "atan", "sinh",  "cosh", "tanh",  "asinh", "acosh", "atanh",
        "log",  "log1p", "log2", "log10", "ceil",  "floor", "trunc",
        "exp",  "exp2",  "abs",  "round",
    };
    const std::vector<std::string> binary_names{"pow", "min", "max"};

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
    }

    SECTION("Arity")
    {
        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto c = Value::create(3_f);

        for (const auto& name : unary_names)
        {
            auto fn = get_fn(name);
            CHECK_THROWS_MATCHES(
                fn->call({}), Frost_User_Error,
                MessageMatches(ContainsSubstring("insufficient arguments")
                               && ContainsSubstring("Called with 0")
                               && ContainsSubstring("requires at least 1")));
            CHECK_THROWS_MATCHES(
                fn->call({a, b}), Frost_User_Error,
                MessageMatches(ContainsSubstring("too many arguments")
                               && ContainsSubstring("Called with 2")
                               && ContainsSubstring("no more than 1")));
        }

        for (const auto& name : binary_names)
        {
            auto fn = get_fn(name);
            CHECK_THROWS_MATCHES(
                fn->call({a}), Frost_User_Error,
                MessageMatches(ContainsSubstring("insufficient arguments")
                               && ContainsSubstring("Called with 1")
                               && ContainsSubstring("requires at least 2")));
            CHECK_THROWS_MATCHES(
                fn->call({a, b, c}), Frost_User_Error,
                MessageMatches(ContainsSubstring("too many arguments")
                               && ContainsSubstring("Called with 3")
                               && ContainsSubstring("no more than 2")));
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
    }
}
