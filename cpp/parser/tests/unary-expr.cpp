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

TEST_CASE("Parser Unary Expressions")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        auto src = lexy::string_input<lexy::utf8_encoding>(input);
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
            auto value = expr->evaluate(table);
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
            auto value = expr->evaluate(table);
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
}
