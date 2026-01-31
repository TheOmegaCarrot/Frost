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

TEST_CASE("Parser Map Literals")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        auto src = lexy::string_input(input);
        return lexy::parse<Expression_Root>(src, lexy::noop);
    };

    SECTION("Empty map literal is valid")
    {
        auto result = parse("{}");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Map>());
        CHECK(out->raw_get<frst::Map>().empty());
    }

    SECTION("Identifier key sugar produces string keys")
    {
        auto result = parse("{foo: 1, bar: 2}");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Map>());
        const auto& map = out->raw_get<frst::Map>();
        REQUIRE(map.size() == 2);

        auto foo_key = frst::Value::create(std::string{"foo"});
        auto bar_key = frst::Value::create(std::string{"bar"});
        auto foo_it = map.find(foo_key);
        auto bar_it = map.find(bar_key);
        REQUIRE(foo_it != map.end());
        REQUIRE(bar_it != map.end());
        CHECK(foo_it->second->get<frst::Int>().value() == 1_f);
        CHECK(bar_it->second->get<frst::Int>().value() == 2_f);
    }

    SECTION("Bracketed key expressions are supported")
    {
        auto result = parse("{[1]: 2, [1+1]: 3}");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Map>());
        const auto& map = out->raw_get<frst::Map>();
        REQUIRE(map.size() == 2);

        auto key1 = frst::Value::create(1_f);
        auto key2 = frst::Value::create(2_f);
        auto it1 = map.find(key1);
        auto it2 = map.find(key2);
        REQUIRE(it1 != map.end());
        REQUIRE(it2 != map.end());
        CHECK(it1->second->get<frst::Int>().value() == 2_f);
        CHECK(it2->second->get<frst::Int>().value() == 3_f);
    }

    SECTION("Mixed key forms are allowed")
    {
        auto result = parse("{foo: 1, [42]: 2, bar: 3}");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Map>());
        const auto& map = out->raw_get<frst::Map>();
        REQUIRE(map.size() == 3);
    }

    SECTION("Trailing commas are allowed")
    {
        auto result = parse("{foo: 1, [2]: 3,}");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Map>());
        CHECK(out->raw_get<frst::Map>().size() == 2);
    }

    SECTION("Commas are required between entries")
    {
        CHECK_FALSE(parse("{foo: 1 bar: 2}"));
        CHECK_FALSE(parse("{a: 1\nb:2}"));
    }

    SECTION("Newlines between entries are allowed with commas")
    {
        auto result = parse("{foo: 1,\nbar: 2}");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Map>());
        CHECK(out->raw_get<frst::Map>().size() == 2);
    }

    SECTION("Map value expressions can span newlines")
    {
        auto result = parse("{foo: 1 +\n 2}");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Map>());
        const auto& map = out->raw_get<frst::Map>();
        auto foo_key = frst::Value::create(std::string{"foo"});
        auto it = map.find(foo_key);
        REQUIRE(it != map.end());
        REQUIRE(it->second->is<frst::Int>());
        CHECK(it->second->get<frst::Int>().value() == 3_f);
    }

    SECTION("Map literals can appear inside arrays")
    {
        auto result = parse("[{a: 1}, {b: 2}]");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 2);
        REQUIRE(arr[0]->is<frst::Map>());
        REQUIRE(arr[1]->is<frst::Map>());
    }

    SECTION("Whitespace between % and { is not allowed")
    {
        CHECK_FALSE(parse("% {}"));
        CHECK_FALSE(parse("% {foo: 1}"));
    }

    SECTION("Identifier key cannot be a keyword")
    {
        CHECK_FALSE(parse("{if: 1}"));
        CHECK_FALSE(parse("{true: 1}"));
    }

    SECTION("Missing brackets for non-identifier keys is invalid")
    {
        CHECK_FALSE(parse("{1: 2}"));
        CHECK_FALSE(parse("{(a): 1}"));
    }

    SECTION("Map literals can be used as atoms")
    {
        auto result = parse("{foo: 1}[\"foo\"]");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 1_f);
    }

    SECTION("Map literals cannot be followed by postfix across newlines")
    {
        auto result = parse("{a: 1}\n[\"a\"]");
        REQUIRE_FALSE(result);
    }

    SECTION("Bracketed identifier keys use the expression value")
    {
        auto result = parse("{[foo]: 1}");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        table.define("foo", frst::Value::create(42_f));
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Map>());
        const auto& map = out->raw_get<frst::Map>();
        auto key = frst::Value::create(42_f);
        REQUIRE(map.find(key) != map.end());
    }

    SECTION("Key expressions can be complex")
    {
        auto result = parse("{[if true: 1 else: 2]: 3}");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Map>());
        const auto& map = out->raw_get<frst::Map>();
        auto key = frst::Value::create(1_f);
        REQUIRE(map.find(key) != map.end());
    }

    SECTION("Value expressions can be complex")
    {
        auto result =
            parse("{a: if true: 1 else: 2, b: fn(x) -> { x }(3), c: [1,2]}");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Map>());
        const auto& map = out->raw_get<frst::Map>();
        auto a_key = frst::Value::create(std::string{"a"});
        auto b_key = frst::Value::create(std::string{"b"});
        auto c_key = frst::Value::create(std::string{"c"});
        REQUIRE(map.find(a_key) != map.end());
        REQUIRE(map.find(b_key) != map.end());
        REQUIRE(map.find(c_key) != map.end());
    }

    SECTION("Complex bracketed key expressions are allowed")
    {
        auto result = parse("{[fn() -> { 1 }()]: 2, [[1,2]]: 3}");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Map>());
    }

    SECTION("Nested map literals are allowed")
    {
        auto result = parse("{a: {b: 1}}");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Map>());
        const auto& map = out->raw_get<frst::Map>();
        auto key = frst::Value::create(std::string{"a"});
        auto it = map.find(key);
        REQUIRE(it != map.end());
        REQUIRE(it->second->is<frst::Map>());
    }

    SECTION("Map literals are valid in postfix and call contexts")
    {
        auto result = parse("{a: 1}[\"a\"]");
        REQUIRE(result);
        auto expr = require_expression(result);
        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 1_f);

        auto result2 = parse("({a: 1}).a");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto out2 = expr2->evaluate(table);
        REQUIRE(out2->is<frst::Int>());
        CHECK(out2->get<frst::Int>().value() == 1_f);

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

        auto callable = std::make_shared<IdentityCallable>();
        table.define("id", frst::Value::create(frst::Function{callable}));

        auto result3 = parse("id({a: 1})");
        REQUIRE(result3);
        auto expr3 = require_expression(result3);
        auto out3 = expr3->evaluate(table);
        REQUIRE(out3->is<frst::Map>());
    }

    SECTION("Trailing comma with comments is accepted")
    {
        auto result = parse("{a: 1, # c\n}");
        REQUIRE(result);
        auto expr = require_expression(result);
        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Map>());
        CHECK(out->raw_get<frst::Map>().size() == 1);
    }

    SECTION("Whitespace inside empty map is allowed")
    {
        auto result = parse("{ }");
        REQUIRE(result);
        auto expr = require_expression(result);
        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Map>());
        CHECK(out->raw_get<frst::Map>().empty());
    }

    SECTION("Malformed maps are rejected")
    {
        CHECK_FALSE(parse("{"));
        CHECK_FALSE(parse("{foo: 1"));
        CHECK_FALSE(parse("{,}"));
        CHECK_FALSE(parse("{foo:1,,bar:2}"));
        CHECK_FALSE(parse("{[1: 2]}"));
        CHECK_FALSE(parse("{[1]:}"));
        CHECK_FALSE(parse("%"));
        CHECK_FALSE(parse("%[]"));
        CHECK_FALSE(parse("{a: 1; b: 2}"));
        CHECK_FALSE(parse("{[1]: 2; [3]: 4}"));
        CHECK_FALSE(parse("{a: 1; }"));
        CHECK_FALSE(parse("{;a: 1}"));
    }
}
