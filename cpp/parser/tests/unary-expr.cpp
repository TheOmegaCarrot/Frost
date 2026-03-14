#include <catch2/catch_test_macros.hpp>

#include <frost/ast.hpp>
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

TEST_CASE("Parser Unary Expressions")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        auto src = lexy::string_input<lexy::utf8_encoding>(input);
        frst::grammar::reset_parse_state(src);
        return lexy::parse<Expression_Root>(src, lexy::noop);
    };

    SECTION("Unary minus evaluates correctly")
    {
        const std::string_view cases[] = {"-1", "-(1)", "- -1", "--1"};
        const frst::Int expected[] = {-1_f, -1_f, 1_f, 1_f};

        for (std::size_t i = 0; i < std::size(cases); ++i)
        {
            auto result = parse(cases[i]);
            REQUIRE(result);
            auto expr = require_expression(result);

            frst::Symbol_Table table;
            frst::Evaluation_Context ctx{.symbols = table};
            auto value = expr->evaluate(ctx);
            REQUIRE(value->is<frst::Int>());
            CHECK(value->get<frst::Int>().value() == expected[i]);
        }
    }

    SECTION("Logical not evaluates correctly")
    {
        struct Case
        {
            std::string_view input;
            bool expected;
        };

        const Case cases[] = {
            {"not true", false},    {"not false", true},   {"not null", true},
            {"not not true", true}, {"not (false)", true}, {"not -1", false},
        };

        for (const auto& c : cases)
        {
            auto result = parse(c.input);
            REQUIRE(result);
            auto expr = require_expression(result);

            frst::Symbol_Table table;
            frst::Evaluation_Context ctx{.symbols = table};
            auto value = expr->evaluate(ctx);
            REQUIRE(value->is<frst::Bool>());
            CHECK(value->get<frst::Bool>().value() == c.expected);
        }
    }

    SECTION("Unary plus is not supported")
    {
        const std::string_view cases[] = {"+1", "+(1)"};

        for (const auto& input : cases)
        {
            auto result = parse(input);
            CHECK_FALSE(result);
            CHECK(result.error_count() >= 1);
        }
    }

    SECTION("Source ranges for unary minus")
    {
        // "-1" → begin at '-' {1,1}, end at '1' {1,2}
        auto result = parse("-1");
        REQUIRE(result);
        auto expr = require_expression(result);
        auto range = expr->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 1);
        CHECK(range.end.line == 1);
        CHECK(range.end.column == 2);
    }

    SECTION("Source ranges for not operator")
    {
        // "not true" → begin at 'n' {1,1}, end at 'e' in true {1,8}
        auto result = parse("not true");
        REQUIRE(result);
        auto expr = require_expression(result);
        auto range = expr->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 1);
        CHECK(range.end.line == 1);
        CHECK(range.end.column == 8);
    }

    SECTION("Source ranges for double negation")
    {
        // "--1" → outer '-' at {1,1}, inner operand '1' at {1,3}
        auto result = parse("--1");
        REQUIRE(result);
        auto expr = require_expression(result);
        auto range = expr->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 1);
        CHECK(range.end.line == 1);
        CHECK(range.end.column == 3);
    }

    SECTION("Source ranges for not with parenthesized operand")
    {
        // "not (1)" → begin at 'n' {1,1}, end at ')' {1,7}
        auto result = parse("not (1)");
        REQUIRE(result);
        auto expr = require_expression(result);
        auto range = expr->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 1);
        CHECK(range.end.line == 1);
        CHECK(range.end.column == 7);
    }
}
