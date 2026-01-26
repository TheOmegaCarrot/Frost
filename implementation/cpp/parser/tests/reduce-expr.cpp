#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/symbol-table.hpp>
#include <frost/testing/stringmaker-specializations.hpp>
#include <frost/value.hpp>

#include <lexy/action/parse.hpp>
#include <lexy/callback.hpp>
#include <lexy/input/string_input.hpp>

#include "../grammar.hpp"

using namespace frst::literals;

namespace
{
struct Expression_Root
{
    static constexpr auto whitespace = frst::grammar::ws;
    static constexpr auto rule =
        lexy::dsl::p<frst::grammar::expression> + lexy::dsl::eof;
    static constexpr auto value = lexy::forward<frst::ast::Expression::Ptr>;
};

frst::ast::Expression::Ptr require_expression(auto& result)
{
    auto expr = std::move(result).value();
    REQUIRE(expr);
    return expr;
}
} // namespace

TEST_CASE("Parser Reduce Expressions")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        auto src = lexy::string_input(input);
        return lexy::parse<Expression_Root>(src, lexy::noop);
    };

    SECTION("Reduce array with init")
    {
        auto result =
            parse("reduce [1, 2, 3] with fn (acc, x) -> { acc + x } init: 0");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 6_f);
    }

    SECTION("Reduce array without init")
    {
        auto result = parse("reduce [1, 2, 3] with fn (acc, x) -> { acc + x }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 6_f);
    }

    SECTION("Reduce empty array with init returns init")
    {
        auto result =
            parse("reduce [] with fn (acc, x) -> { acc + x } init: 5");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 5_f);
    }

    SECTION("Reduce map without init is an evaluation error")
    {
        auto result = parse("reduce %{a: 1} with fn (acc, k, v) -> { acc }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        CHECK_THROWS_WITH(expr->evaluate(table),
                          Catch::Matchers::ContainsSubstring("init"));
    }

    SECTION("Reduce expressions can participate in larger expressions")
    {
        auto result =
            parse("(reduce [1, 2] with fn (acc, x) -> { acc + x }) + 1");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 4_f);
    }

    SECTION("Reduce results can be chained in larger expressions")
    {
        auto result =
            parse("(reduce [1, 2, 3] with fn (acc, x) -> { acc + x }) * 2");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 12_f);

        auto result2 =
            parse("(reduce [] with fn (acc, x) -> { acc + x } init: 4) - 1");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto out2 = expr2->evaluate(table);
        REQUIRE(out2->is<frst::Int>());
        CHECK(out2->get<frst::Int>().value() == 3_f);

        auto result3 = parse(
            "(reduce %{a: 1} with fn (acc, k, v) -> { acc } init: 5) + 1");
        REQUIRE(result3);
        auto expr3 = require_expression(result3);
        auto out3 = expr3->evaluate(table);
        REQUIRE(out3->is<frst::Int>());
        CHECK(out3->get<frst::Int>().value() == 6_f);
    }

    SECTION("Whitespace and comments are allowed around reduce init")
    {
        auto result =
            parse("reduce [1, 2] with fn (acc, x) -> { acc + x } init:\n0");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 3_f);

        auto result2 =
            parse("reduce [1, 2] with fn (acc, x) -> { acc + x } init : 0");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto out2 = expr2->evaluate(table);
        REQUIRE(out2->is<frst::Int>());
        CHECK(out2->get<frst::Int>().value() == 3_f);
    }

    SECTION("Reduce expressions can appear inside other constructs")
    {
        auto result = parse("[reduce [1, 2] with fn (acc, x) -> { acc + x }]");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 1);
        CHECK(arr[0]->get<frst::Int>().value() == 3_f);

        auto result2 =
            parse("%{[reduce [1, 2] with fn (acc, x) -> { acc + x }]: 1}");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto out2 = expr2->evaluate(table);
        REQUIRE(out2->is<frst::Map>());
        auto key = frst::Value::create(3_f);
        auto it = out2->raw_get<frst::Map>().find(key);
        REQUIRE(it != out2->raw_get<frst::Map>().end());

        auto result3 = parse(
            "if reduce [1, 2] with fn (acc, x) -> { acc + x }: 1 else: 2");
        REQUIRE(result3);
        auto expr3 = require_expression(result3);
        auto out3 = expr3->evaluate(table);
        REQUIRE(out3->is<frst::Int>());
        CHECK(out3->get<frst::Int>().value() == 1_f);
    }

    SECTION("Reduce does not bind across newlines in larger expressions")
    {
        auto result =
            parse("(reduce [1, 2] with fn (acc, x) -> { acc + x })\n+ 1");
        REQUIRE_FALSE(result);
    }

    SECTION("Reduce expressions can nest other higher-order expressions")
    {
        auto result = parse("reduce (map [1, 2] with fn (x) -> { x }) "
                            "with fn (acc, x) -> { acc + x }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 3_f);

        auto result2 = parse("reduce [1, 2] with fn (acc, x) -> { "
                             "(map [x] with fn (y) -> { y })[0] + acc }");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto out2 = expr2->evaluate(table);
        REQUIRE(out2->is<frst::Int>());
        CHECK(out2->get<frst::Int>().value() == 3_f);

        auto result3 = parse("reduce [1, 2] with fn (acc, x) -> { acc + x } "
                             "init: (map [3] with fn (x) -> { x })[0]");
        REQUIRE(result3);
        auto expr3 = require_expression(result3);
        auto out3 = expr3->evaluate(table);
        REQUIRE(out3->is<frst::Int>());
        CHECK(out3->get<frst::Int>().value() == 6_f);

        auto result4 =
            parse("reduce %{a: 1} with fn (acc, k, v) -> { acc } "
                  "init: (reduce [1, 2] with fn (acc, x) -> { acc + x })");
        REQUIRE(result4);
        auto expr4 = require_expression(result4);
        auto out4 = expr4->evaluate(table);
        REQUIRE(out4->is<frst::Int>());
        CHECK(out4->get<frst::Int>().value() == 3_f);
    }

    SECTION("Init expressions can include nested map and UFCS")
    {
        struct Identity_Callable final : frst::Callable
        {
            frst::Value_Ptr call(
                std::span<const frst::Value_Ptr> args) const override
            {
                if (args.empty())
                    return frst::Value::null();
                return args.front();
            }

            std::string debug_dump() const override
            {
                return "<identity>";
            }
        };

        auto result = parse("reduce [1] with fn (acc, x) -> { acc + x } "
                            "init: (map [1] with f @ g())[0]");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto f_callable = std::make_shared<Identity_Callable>();
        auto g_callable = std::make_shared<Identity_Callable>();
        table.define("f", frst::Value::create(frst::Function{f_callable}));
        table.define("g", frst::Value::create(frst::Function{g_callable}));

        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 2_f);
    }

    SECTION("Reduce expressions can be passed as call arguments")
    {
        struct IdentityCallable final : frst::Callable
        {
            frst::Value_Ptr call(
                std::span<const frst::Value_Ptr> args) const override
            {
                if (args.empty())
                {
                    return frst::Value::null();
                }
                return args.front();
            }

            std::string debug_dump() const override
            {
                return "<identity>";
            }
        };

        auto result =
            parse("f(reduce [1, 2, 3] with fn (acc, x) -> { acc + x })");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto callable = std::make_shared<IdentityCallable>();
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 6_f);
    }

    SECTION("UFCS can apply to reduce expressions")
    {
        struct IdentityCallable final : frst::Callable
        {
            frst::Value_Ptr call(
                std::span<const frst::Value_Ptr> args) const override
            {
                if (args.empty())
                {
                    return frst::Value::null();
                }
                return args.front();
            }

            std::string debug_dump() const override
            {
                return "<identity>";
            }
        };

        auto result =
            parse("reduce [1, 2] with fn (acc, x) -> { acc + x } @ f()");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto callable = std::make_shared<IdentityCallable>();
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 3_f);
    }

    SECTION("Reduce can consume a filtered structure and map init")
    {
        auto result =
            parse("reduce (filter [1, 2, 3] with fn (x) -> { x > 1 }) "
                  "with fn (acc, x) -> { acc + x } "
                  "init: (map [1] with fn (x) -> { x })[0]");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 6_f);
    }

    SECTION("Nested reduce in operation expression is parsed correctly")
    {
        auto result = parse("reduce a with reduce b with f init: c");
        REQUIRE(result);
        auto expr = require_expression(result);

        struct Return_Acc final : frst::Callable
        {
            frst::Value_Ptr call(
                std::span<const frst::Value_Ptr> args) const override
            {
                return args.front();
            }

            std::string debug_dump() const override
            {
                return "<return-acc>";
            }
        };

        struct Recording_Acc final : frst::Callable
        {
            mutable std::vector<std::vector<frst::Value_Ptr>> calls;

            frst::Value_Ptr call(
                std::span<const frst::Value_Ptr> args) const override
            {
                calls.emplace_back(args.begin(), args.end());
                return args.front();
            }

            std::string debug_dump() const override
            {
                return "<recording-acc>";
            }
        };

        frst::Symbol_Table table;
        auto a_val = frst::Value::create(frst::Array{
            frst::Value::create(10_f),
            frst::Value::create(20_f),
        });
        auto b_val = frst::Value::create(frst::Array{
            frst::Value::create(1_f),
        });
        auto f_callable = std::make_shared<Return_Acc>();
        auto g_callable = std::make_shared<Recording_Acc>();
        auto c_val = frst::Value::create(frst::Function{g_callable});

        table.define("a", a_val);
        table.define("b", b_val);
        table.define("f", frst::Value::create(frst::Function{f_callable}));
        table.define("c", c_val);

        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 10_f);
        REQUIRE(g_callable->calls.size() == 1);
        CHECK(g_callable->calls[0][0]->get<frst::Int>().value() == 10_f);
        CHECK(g_callable->calls[0][1]->get<frst::Int>().value() == 20_f);
    }

    SECTION("Invalid reduce expressions fail to parse")
    {
        const std::string_view cases[] = {
            "reduce with f",
            "reduce [1] init: 0 with f",
            "reduce [1] with f init",
            "reduce [1] with f init:",
            "reduce [1] with f init: 0 init: 1",
        };

        for (const auto& input : cases)
        {
            auto result = parse(input);
            CHECK_FALSE(result);
            CHECK(result.error_count() >= 1);
        }
    }
}
