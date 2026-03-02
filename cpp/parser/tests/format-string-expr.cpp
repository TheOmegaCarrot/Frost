#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_exception.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/symbol-table.hpp>
#include <frost/testing/stringmaker-specializations.hpp>
#include <frost/value.hpp>

#include <lexy/action/parse.hpp>
#include <lexy/callback.hpp>
#include <lexy/input/string_input.hpp>

#include "../grammar.hpp"

using namespace frst::literals;
using Catch::Matchers::ContainsSubstring;
using Catch::Matchers::MessageMatches;

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

TEST_CASE("Parser Format Strings")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        auto src = lexy::string_input<lexy::utf8_encoding>(input);
        return lexy::parse<Expression_Root>(src, lexy::noop);
    };

    SECTION("Double- and single-quoted format strings parse")
    {
        const std::string_view cases[] = {
            R"($"hello")",
            R"($'hello')",
        };

        for (const auto& input : cases)
        {
            auto result = parse(input);
            REQUIRE(result);
            auto expr = require_expression(result);

            frst::Symbol_Table table;
            auto value = expr->evaluate(table);
            REQUIRE(value->is<frst::String>());
            CHECK(value->get<frst::String>().value() == "hello");
        }
    }

    SECTION("Placeholders evaluate against the symbol table")
    {
        auto result = parse(R"($"value ${x}")");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        table.define("x", frst::Value::create(42_f));

        auto value = expr->evaluate(table);
        REQUIRE(value->is<frst::String>());
        CHECK(value->get<frst::String>().value() == "value 42");
    }

    SECTION("Escaped placeholder is treated as literal")
    {
        auto result = parse(R"($"\\${x}")");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        table.define("x", frst::Value::create(1_f));

        auto value = expr->evaluate(table);
        REQUIRE(value->is<frst::String>());
        CHECK(value->get<frst::String>().value() == "${x}");
    }

    SECTION("Raw strings are not permitted in format strings")
    {
        const std::string_view cases[] = {
            "$R\"(hello)\"",
            "$R'(hello)'",
        };

        for (const auto& input : cases)
        {
            auto result = parse(input);
            CHECK_FALSE(result);
        }
    }

    SECTION("Whitespace between $ and quote is not permitted")
    {
        auto result = parse(R"($ "hello")");
        CHECK_FALSE(result);
    }

    SECTION("Non-string tokens after $ are rejected")
    {
        auto result = parse("$42");
        CHECK_FALSE(result);
    }

    SECTION("Adjacent expressions after a format string are rejected")
    {
        auto result = parse(R"($"x"y)");
        CHECK_FALSE(result);
    }

    SECTION("Malformed placeholders throw during parse")
    {
        CHECK_THROWS(parse(R"($"${}")"));
        CHECK_THROWS(parse(R"($"${1}")"));
        CHECK_THROWS(parse(R"($"${foo${bar}}")"));
    }

    SECTION("Missing name lookup propagates as an error")
    {
        auto result = parse(R"($"${missing}")");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        CHECK_THROWS_MATCHES(expr->evaluate(table),
                             frst::Frost_Unrecoverable_Error,
                             MessageMatches(ContainsSubstring("missing")));
    }

    SECTION("Format string inside a lambda expression parses")
    {
        auto result = parse(R"(fn () -> { $"hello ${name}" })");
        CHECK(result);
    }

    SECTION("Format string in composite expressions parses")
    {
        const std::string_view cases[] = {
            R"([$"hi"])", R"({ key: $"hi" })", R"($"a" == $"b")",
            R"($"x"(1))", R"($"x"[0])",
        };

        for (const auto& input : cases)
        {
            auto result = parse(input);
            CHECK(result);
        }
    }
}
