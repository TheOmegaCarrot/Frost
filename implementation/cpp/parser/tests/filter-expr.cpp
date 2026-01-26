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

TEST_CASE("Parser Filter Expressions")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        auto src = lexy::string_input(input);
        return lexy::parse<Expression_Root>(src, lexy::noop);
    };

    SECTION("Filter array with a predicate")
    {
        auto result = parse("filter [1, 2, 3] with fn (x) -> { x > 1 }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 2);
        CHECK(arr[0]->get<frst::Int>().value() == 2_f);
        CHECK(arr[1]->get<frst::Int>().value() == 3_f);
    }

    SECTION("Filter map with a predicate")
    {
        auto result = parse("filter %{a: 1, b: 2} with fn (k, v) -> { v > 1 }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Map>());
        const auto& map = out->raw_get<frst::Map>();
        REQUIRE(map.size() == 1);

        auto b_key = frst::Value::create(std::string{"b"});
        auto it = map.find(b_key);
        REQUIRE(it != map.end());
        CHECK(it->second->get<frst::Int>().value() == 2_f);
    }

    SECTION("Filter expressions can be postfixed")
    {
        auto result = parse("(filter [1, 2, 3] with fn (x) -> { x > 1 })[0]");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 2_f);
    }

    SECTION("Filter expressions can be used inside larger expressions")
    {
        auto result =
            parse("(filter [1, 2, 3] with fn (x) -> { x > 1 })[0] + 5");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 7_f);

        auto result2 =
            parse("(filter [1, 2, 3] with fn (x) -> { x > 2 })[0] == 3");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto out2 = expr2->evaluate(table);
        REQUIRE(out2->is<frst::Bool>());
        CHECK(out2->get<frst::Bool>().value() == true);
    }

    SECTION("Whitespace and comments are allowed around filter keywords")
    {
        auto result = parse("filter\n[1, 2]\nwith\nfn (x) -> { x > 1 }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 1);
        CHECK(arr[0]->get<frst::Int>().value() == 2_f);
    }

    SECTION("Filter expressions can appear inside other constructs")
    {
        auto result = parse("[filter [1, 2] with fn (x) -> { x > 1 }]");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Array>());
        const auto& outer = out->raw_get<frst::Array>();
        REQUIRE(outer.size() == 1);
        REQUIRE(outer[0]->is<frst::Array>());
        CHECK(outer[0]->raw_get<frst::Array>().size() == 1);

        auto result2 = parse("%{k: filter [1, 2] with fn (x) -> { x > 1 }}");
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

        auto result3 =
            parse("if filter [1, 2] with fn (x) -> { x > 1 }: 1 else: 2");
        REQUIRE(result3);
        auto expr3 = require_expression(result3);
        auto out3 = expr3->evaluate(table);
        REQUIRE(out3->is<frst::Int>());
        CHECK(out3->get<frst::Int>().value() == 1_f);
    }

    SECTION("Postfix does not bind across newlines after filter expressions")
    {
        auto result = parse("(filter [1, 2, 3] with fn (x) -> { x > 1 })\n[0]");
        REQUIRE_FALSE(result);
    }

    SECTION("Filter expressions can nest other higher-order expressions")
    {
        auto result = parse("filter (map [1, 2, 3] with fn (x) -> { x }) "
                            "with fn (x) -> { x > 1 }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 2);
        CHECK(arr[0]->get<frst::Int>().value() == 2_f);
        CHECK(arr[1]->get<frst::Int>().value() == 3_f);

        auto result2 =
            parse("filter [1, 2, 3] with fn (x) -> { "
                  "(reduce [1, 2] with fn (acc, y) -> { acc + y }) > 2 }");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto out2 = expr2->evaluate(table);
        REQUIRE(out2->is<frst::Array>());
        const auto& arr2 = out2->raw_get<frst::Array>();
        REQUIRE(arr2.size() == 3);
        CHECK(arr2[0]->get<frst::Int>().value() == 1_f);
        CHECK(arr2[1]->get<frst::Int>().value() == 2_f);
        CHECK(arr2[2]->get<frst::Int>().value() == 3_f);

        auto result3 = parse("filter [1, 2] with fn (x) -> { "
                             "foreach [1] with fn (y) -> { y }; true }");
        REQUIRE(result3);
        auto expr3 = require_expression(result3);
        auto out3 = expr3->evaluate(table);
        REQUIRE(out3->is<frst::Array>());
        const auto& arr3 = out3->raw_get<frst::Array>();
        REQUIRE(arr3.size() == 2);
        CHECK(arr3[0]->get<frst::Int>().value() == 1_f);
        CHECK(arr3[1]->get<frst::Int>().value() == 2_f);
    }

    SECTION("Filter expressions can be passed as call arguments")
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

        auto result = parse("f(filter [1, 2, 3] with fn (x) -> { x > 1 })");
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

    SECTION("Filter predicates can embed nested map expressions")
    {
        auto result = parse("filter [1, 2, 3] with fn (x) -> { "
                            "(map [x] with fn (y) -> { y + 1 })[0] > 2 }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 2);
        CHECK(arr[0]->get<frst::Int>().value() == 2_f);
        CHECK(arr[1]->get<frst::Int>().value() == 3_f);
    }

    SECTION("Invalid filter expressions fail to parse")
    {
        const std::string_view cases[] = {
            "filter with f",        "filter [1] f",
            "filter [1] with",      "filter [1] with f init: 0",
            "filter [1] with init", "filter [1] with fn (map) -> { map }",
        };

        for (const auto& input : cases)
        {
            auto result = parse(input);
            CHECK_FALSE(result);
            CHECK(result.error_count() >= 1);
        }
    }
}
