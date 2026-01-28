#include <catch2/catch_test_macros.hpp>

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

TEST_CASE("Parser Map Expressions")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        auto src = lexy::string_input(input);
        return lexy::parse<Expression_Root>(src, lexy::noop);
    };

    SECTION("Map array with a lambda operation")
    {
        auto result = parse("map [1, 2, 3] with fn (x) -> { x + 1 }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 3);
        CHECK(arr[0]->get<frst::Int>().value() == 2_f);
        CHECK(arr[1]->get<frst::Int>().value() == 3_f);
        CHECK(arr[2]->get<frst::Int>().value() == 4_f);
    }

    SECTION("Map map with a lambda operation")
    {
        auto result =
            parse("map {a: 1, b: 2} with fn (k, v) -> { {[k]: v + 1} }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Map>());
        const auto& map = out->raw_get<frst::Map>();
        REQUIRE(map.size() == 2);

        auto a_key = frst::Value::create(std::string{"a"});
        auto b_key = frst::Value::create(std::string{"b"});
        auto a_it = map.find(a_key);
        auto b_it = map.find(b_key);
        REQUIRE(a_it != map.end());
        REQUIRE(b_it != map.end());
        CHECK(a_it->second->get<frst::Int>().value() == 2_f);
        CHECK(b_it->second->get<frst::Int>().value() == 3_f);
    }

    SECTION("Map expressions can be postfixed")
    {
        auto result = parse("(map [1, 2] with fn (x) -> { x })[0]");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 1_f);
    }

    SECTION("Map expressions can be used inside larger expressions")
    {
        auto result = parse("(map [1, 2] with fn (x) -> { x + 1 })[1] * 2");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 6_f);

        auto result2 = parse("(map [1] with fn (x) -> { x })[0] == 1");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto out2 = expr2->evaluate(table);
        REQUIRE(out2->is<frst::Bool>());
        CHECK(out2->get<frst::Bool>().value() == true);
    }

    SECTION("Whitespace and comments are allowed around map keywords")
    {
        auto result = parse("map\n[1]\nwith\nfn (x) -> { x }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 1);
        CHECK(arr[0]->get<frst::Int>().value() == 1_f);
    }

    SECTION("Map expressions can appear inside other constructs")
    {
        auto result = parse("[map [1] with fn (x) -> { x }]");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Array>());
        const auto& outer = out->raw_get<frst::Array>();
        REQUIRE(outer.size() == 1);
        REQUIRE(outer[0]->is<frst::Array>());
        CHECK(outer[0]->raw_get<frst::Array>().size() == 1);

        auto result2 = parse("{k: map [1] with fn (x) -> { x + 1 }}");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto out2 = expr2->evaluate(table);
        REQUIRE(out2->is<frst::Map>());
        auto key = frst::Value::create(std::string{"k"});
        auto it = out2->raw_get<frst::Map>().find(key);
        REQUIRE(it != out2->raw_get<frst::Map>().end());
        REQUIRE(it->second->is<frst::Array>());
        CHECK(it->second->raw_get<frst::Array>().at(0)->get<frst::Int>().value()
              == 2_f);

        auto result3 = parse("if map [1] with fn (x) -> { x }: 1 else: 2");
        REQUIRE(result3);
        auto expr3 = require_expression(result3);
        auto out3 = expr3->evaluate(table);
        REQUIRE(out3->is<frst::Int>());
        CHECK(out3->get<frst::Int>().value() == 1_f);
    }

    SECTION("Postfix does not bind across newlines after map expressions")
    {
        auto result = parse("(map [1] with fn (x) -> { x })\n[0]");
        REQUIRE_FALSE(result);
    }

    SECTION("Map expressions can nest other higher-order expressions")
    {
        auto result = parse(
            "map (map [1, 2] with fn (x) -> { x }) with fn (x) -> { x + 1 }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 2);
        CHECK(arr[0]->get<frst::Int>().value() == 2_f);
        CHECK(arr[1]->get<frst::Int>().value() == 3_f);

        auto result2 = parse("map (filter [1, 2, 3] with fn (x) -> { x > 1 }) "
                             "with fn (x) -> { x * 2 }");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto out2 = expr2->evaluate(table);
        REQUIRE(out2->is<frst::Array>());
        const auto& arr2 = out2->raw_get<frst::Array>();
        REQUIRE(arr2.size() == 2);
        CHECK(arr2[0]->get<frst::Int>().value() == 4_f);
        CHECK(arr2[1]->get<frst::Int>().value() == 6_f);

        auto result3 = parse("map [1, 2] with fn (x) -> { (reduce [1, 2] with "
                             "fn (acc, y) -> { acc + y }) + x }");
        REQUIRE(result3);
        auto expr3 = require_expression(result3);
        auto out3 = expr3->evaluate(table);
        REQUIRE(out3->is<frst::Array>());
        const auto& arr3 = out3->raw_get<frst::Array>();
        REQUIRE(arr3.size() == 2);
        CHECK(arr3[0]->get<frst::Int>().value() == 4_f);
        CHECK(arr3[1]->get<frst::Int>().value() == 5_f);

        auto result4 = parse(
            "map [1] with fn (x) -> { foreach [1] with fn (y) -> { y }; x }");
        REQUIRE(result4);
        auto expr4 = require_expression(result4);
        auto out4 = expr4->evaluate(table);
        REQUIRE(out4->is<frst::Array>());
        const auto& arr4 = out4->raw_get<frst::Array>();
        REQUIRE(arr4.size() == 1);
        CHECK(arr4[0]->get<frst::Int>().value() == 1_f);
    }

    SECTION("Map expressions can be passed as call arguments")
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

        auto result = parse("f(map [1, 2] with fn (x) -> { x + 1 })");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto callable = std::make_shared<IdentityCallable>();
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 2);
        CHECK(arr[0]->get<frst::Int>().value() == 2_f);
        CHECK(arr[1]->get<frst::Int>().value() == 3_f);
    }

    SECTION("Map lambdas can use UFCS within their bodies")
    {
        struct WrapCallable final : frst::Callable
        {
            frst::Value_Ptr call(
                std::span<const frst::Value_Ptr> args) const override
            {
                if (args.empty())
                {
                    return frst::Value::null();
                }
                return frst::Value::create(frst::Array{args.front()});
            }

            std::string debug_dump() const override
            {
                return "<wrap>";
            }
        };

        auto result = parse("map [1, 2] with fn (x) -> { (x @ f())[0] }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto callable = std::make_shared<WrapCallable>();
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 2);
        CHECK(arr[0]->get<frst::Int>().value() == 1_f);
        CHECK(arr[1]->get<frst::Int>().value() == 2_f);
    }

    SECTION("Invalid map expressions fail to parse")
    {
        const std::string_view cases[] = {
            "map with f",       "map [1] f",
            "map [1] with",     "map [1] with f init: 0",
            "map [1] with map", "map [1] with fn (init) -> { init }",
        };

        for (const auto& input : cases)
        {
            auto result = parse(input);
            CHECK_FALSE(result);
            CHECK(result.error_count() >= 1);
        }
    }
}
