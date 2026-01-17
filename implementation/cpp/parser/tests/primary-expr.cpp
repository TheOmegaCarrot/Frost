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

TEST_CASE("Parser Primary Expressions")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        auto src = lexy::string_input(input);
        return lexy::parse<Expression_Root>(src, lexy::noop);
    };

    SECTION("Literals evaluate correctly")
    {
        struct Int_Case
        {
            std::string_view input;
            frst::Int expected;
        };

        const Int_Case int_cases[] = {
            {"0", 0_f},
            {"42", 42_f},
        };

        for (const auto& c : int_cases)
        {
            auto result = parse(c.input);
            REQUIRE(result);
            auto expr = require_expression(result);

            frst::Symbol_Table table;
            auto value = expr->evaluate(table);
            REQUIRE(value->is<frst::Int>());
            CHECK(value->get<frst::Int>().value() == c.expected);
        }

        struct Float_Case
        {
            std::string_view input;
            frst::Float expected;
        };

        const Float_Case float_cases[] = {
            {"1.5", 1.5},
            {"10.25", 10.25},
        };

        for (const auto& c : float_cases)
        {
            auto result = parse(c.input);
            REQUIRE(result);
            auto expr = require_expression(result);

            frst::Symbol_Table table;
            auto value = expr->evaluate(table);
            REQUIRE(value->is<frst::Float>());
            CHECK(value->get<frst::Float>().value() == c.expected);
        }

        struct String_Case
        {
            std::string_view input;
            std::string_view expected;
        };

        const String_Case string_cases[] = {
            {"\"hi\"", "hi"},
            {"\"a b\"", "a b"},
        };

        for (const auto& c : string_cases)
        {
            auto result = parse(c.input);
            REQUIRE(result);
            auto expr = require_expression(result);

            frst::Symbol_Table table;
            auto value = expr->evaluate(table);
            REQUIRE(value->is<frst::String>());
            CHECK(value->get<frst::String>().value() == c.expected);
        }

        const std::string_view bool_cases[] = {"true", "false"};
        const bool bool_expected[] = {true, false};

        for (std::size_t i = 0; i < std::size(bool_cases); ++i)
        {
            auto result = parse(bool_cases[i]);
            REQUIRE(result);
            auto expr = require_expression(result);

            frst::Symbol_Table table;
            auto value = expr->evaluate(table);
            REQUIRE(value->is<frst::Bool>());
            CHECK(value->get<frst::Bool>().value() == bool_expected[i]);
        }

        auto null_result = parse("null");
        REQUIRE(null_result);
        auto null_expr = require_expression(null_result);
        frst::Symbol_Table table;
        auto null_value = null_expr->evaluate(table);
        REQUIRE(null_value->is<frst::Null>());
    }

    SECTION("Name lookup evaluates against the symbol table")
    {
        auto result = parse("foo");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto value = frst::Value::create(42_f);
        table.define("foo", value);

        auto evaluated = expr->evaluate(table);
        CHECK(evaluated == value);
    }

    SECTION("Parenthesized expressions parse and evaluate")
    {
        auto int_result = parse("(  7 )");
        REQUIRE(int_result);
        auto int_expr = require_expression(int_result);

        frst::Symbol_Table table;
        auto int_value = int_expr->evaluate(table);
        REQUIRE(int_value->is<frst::Int>());
        CHECK(int_value->get<frst::Int>().value() == 7_f);

        auto nested_result = parse("((name))");
        REQUIRE(nested_result);
        auto nested_expr = require_expression(nested_result);

        auto stored = frst::Value::create(123_f);
        table.define("name", stored);
        auto nested_value = nested_expr->evaluate(table);
        CHECK(nested_value == stored);

        auto comment_result = parse("(# comment\nname)");
        REQUIRE(comment_result);
        auto comment_expr = require_expression(comment_result);
        auto comment_value = comment_expr->evaluate(table);
        CHECK(comment_value == stored);
    }

    SECTION("Invalid parenthesized expressions fail to parse")
    {
        const std::string_view cases[] = {
            "()",
            "(",
            ")",
            "(1",
            "1)",
            "((1)",
            "(1))",
        };

        for (const auto& input : cases)
        {
            auto result = parse(input);
            CHECK_FALSE(result);
            CHECK(result.error_count() >= 1);
        }
    }

    SECTION("Name lookup failure propagates as an error")
    {
        auto result = parse("missing");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        CHECK_THROWS_WITH(expr->evaluate(table),
                          Catch::Matchers::ContainsSubstring("not defined"));
        CHECK_THROWS_WITH(expr->evaluate(table),
                          Catch::Matchers::ContainsSubstring("missing"));
    }

    SECTION("Reserved keywords fail as expressions")
    {
        const std::string_view cases[] = {"def", "if", "else", "elif"};

        for (const auto& input : cases)
        {
            auto result = parse(input);
            CHECK_FALSE(result);
            CHECK(result.error_count() >= 1);
        }
    }

    SECTION("Expression parsing rejects extra tokens")
    {
        const std::string_view cases[] = {
            "(true)false",
            "name name",
            "null true",
            "\"hi\" \"there\"",
        };

        for (const auto& input : cases)
        {
            auto result = parse(input);
            CHECK_FALSE(result);
            CHECK(result.error_count() >= 1);
        }
    }

    SECTION("Parenthesized lookup failure propagates")
    {
        auto result = parse("(missing)");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        CHECK_THROWS_WITH(expr->evaluate(table),
                          Catch::Matchers::ContainsSubstring("not defined"));
        CHECK_THROWS_WITH(expr->evaluate(table),
                          Catch::Matchers::ContainsSubstring("missing"));
    }

    SECTION("Keyword prefixes parse as identifiers")
    {
        const std::string_view cases[] = {"truex", "null_", "if1"};

        for (const auto& input : cases)
        {
            auto result = parse(input);
            REQUIRE(result);
            auto expr = require_expression(result);

            frst::Symbol_Table table;
            auto value = frst::Value::create(1_f);
            table.define(std::string{input}, value);
            auto evaluated = expr->evaluate(table);
            CHECK(evaluated == value);
        }
    }

    SECTION("Null literal returns the singleton")
    {
        auto result = parse("null");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto value = expr->evaluate(table);
        CHECK(value == frst::Value::null());
    }
}
