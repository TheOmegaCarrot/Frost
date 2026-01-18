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

TEST_CASE("Parser Binary Expressions")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        auto src = lexy::string_input(input);
        return lexy::parse<Expression_Root>(src, lexy::noop);
    };

    SECTION("Arithmetic precedence")
    {
        struct Case
        {
            std::string_view input;
            frst::Int expected;
        };

        const Case cases[] = {
            {"1+2*3", 7_f},   {"1*2+3", 5_f},   {"1+2*3+4", 11_f},
            {"(1+2)*3", 9_f}, {"1*(2+3)", 5_f}, {"1+2*3*4", 25_f},
        };

        for (const auto& c : cases)
        {
            auto result = parse(c.input);
            REQUIRE(result);
            auto expr = require_expression(result);

            frst::Symbol_Table table;
            auto value = expr->evaluate(table);
            REQUIRE(value->is<frst::Int>());
            CHECK(value->get<frst::Int>().value() == c.expected);
        }
    }

    SECTION("Arithmetic associativity")
    {
        struct Case
        {
            std::string_view input;
            frst::Int expected;
        };

        const Case cases[] = {
            {"10-3-2", 5_f},
            {"8/2/2", 2_f},
            {"20-5-5-5", 5_f},
        };

        for (const auto& c : cases)
        {
            auto result = parse(c.input);
            REQUIRE(result);
            auto expr = require_expression(result);

            frst::Symbol_Table table;
            auto value = expr->evaluate(table);
            REQUIRE(value->is<frst::Int>());
            CHECK(value->get<frst::Int>().value() == c.expected);
        }
    }

    SECTION("Unary binds tighter than binary")
    {
        struct Case
        {
            std::string_view input;
            frst::Int expected;
        };

        const Case cases[] = {
            {"-1*2", -2_f},
            {"1+-2", -1_f},
            {"1*-2", -2_f},
            {"-1-2", -3_f},
        };

        for (const auto& c : cases)
        {
            auto result = parse(c.input);
            REQUIRE(result);
            auto expr = require_expression(result);

            frst::Symbol_Table table;
            auto value = expr->evaluate(table);
            REQUIRE(value->is<frst::Int>());
            CHECK(value->get<frst::Int>().value() == c.expected);
        }
    }

    SECTION("Comparison and equality precedence")
    {
        struct Case
        {
            std::string_view input;
            bool expected;
        };

        const Case cases[] = {
            {"1<2==true", true}, {"1<2==false", false}, {"1+1<3", true},
            {"1+1<2", false},    {"2*2>=3+1", true},
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

    SECTION("Mixed comparison and equality are permitted")
    {
        struct Case
        {
            std::string_view input;
            bool expected;
        };

        const Case cases[] = {
            {"1<2==1", false},
            {"1==2<3", false},
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

    SECTION("Logical operator precedence")
    {
        struct Case
        {
            std::string_view input;
            bool expected;
        };

        const Case cases[] = {
            {"true or false and false", true},
            {"false or true and false", false},
            {"false and true or true", true},
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

    SECTION("Logical operators return operands and short-circuit")
    {
        frst::Symbol_Table table;
        auto value = frst::Value::create(123_f);
        table.define("value", value);

        auto or_result = parse("false or value");
        REQUIRE(or_result);
        auto or_expr = require_expression(or_result);
        auto or_value = or_expr->evaluate(table);
        CHECK(or_value == value);

        auto and_result = parse("false and missing");
        REQUIRE(and_result);
        auto and_expr = require_expression(and_result);
        auto and_value = and_expr->evaluate(table);
        REQUIRE(and_value->is<frst::Bool>());
        CHECK(and_value->get<frst::Bool>().value() == false);

        auto or_short = parse("true or missing");
        REQUIRE(or_short);
        auto or_short_expr = require_expression(or_short);
        auto or_short_value = or_short_expr->evaluate(table);
        REQUIRE(or_short_value->is<frst::Bool>());
        CHECK(or_short_value->get<frst::Bool>().value() == true);
    }

    SECTION("Chained comparisons and equality are syntax errors")
    {
        const std::string_view cases[] = {
            "1<2<3",   "1<=2<=3", "1>2>3",   "1==2==3",
            "1!=2!=3", "1<2>=3",  "1==2!=3", "1<=2>3",
        };

        for (const auto& input : cases)
        {
            auto result = parse(input);
            CHECK_FALSE(result);
            CHECK(result.error_count() >= 1);
        }
    }

    SECTION("Operator tokenization respects boundaries")
    {
        struct Case
        {
            std::string_view input;
            bool expected;
        };

        const Case cases[] = {
            {"1<=2", true},
            {"1==2", false},
            {"2==2", true},
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

        const std::string_view invalid_cases[] = {
            "1< =2",
            "1= =2",
            "1<==2",
        };

        for (const auto& input : invalid_cases)
        {
            auto result = parse(input);
            CHECK_FALSE(result);
            CHECK(result.error_count() >= 1);
        }
    }

    SECTION("Keyword operators do not consume identifier suffixes")
    {
        struct Case
        {
            std::string_view input;
            bool left_value;
        };

        const Case cases[] = {
            {"and1 or or2", false},
            {"and1 and or2", true},
        };

        for (const auto& c : cases)
        {
            auto result = parse(c.input);
            REQUIRE(result);
            auto expr = require_expression(result);

            frst::Symbol_Table table;
            auto left = frst::Value::create(auto{c.left_value});
            auto right = frst::Value::create(7_f);
            table.define("and1", left);
            table.define("or2", right);

            auto value = expr->evaluate(table);
            CHECK(value == right);
        }
    }

    SECTION("Logical operators with non-boolean operands return operands")
    {
        struct Case
        {
            std::string_view input;
            frst::Int expected;
        };

        const Case cases[] = {
            {"1==1 and 99", 99_f},
            {"0==1 or 7", 7_f},
        };

        for (const auto& c : cases)
        {
            auto result = parse(c.input);
            REQUIRE(result);
            auto expr = require_expression(result);

            frst::Symbol_Table table;
            auto value = expr->evaluate(table);
            REQUIRE(value->is<frst::Int>());
            CHECK(value->get<frst::Int>().value() == c.expected);
        }
    }

    SECTION("Parenthesized grouping across operator levels")
    {
        struct Case
        {
            std::string_view input;
            bool expected;
        };

        const Case cases[] = {
            {"(1<2)==true", true},
            {"(1<2)==(2<3)", true},
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
}
