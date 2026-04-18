#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

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
struct Integer_Root
{
    static constexpr auto whitespace = frst::grammar::ws;
    static constexpr auto rule =
        lexy::dsl::p<frst::grammar::integer_literal> + lexy::dsl::eof;
    static constexpr auto value = lexy::forward<frst::Value_Ptr>;
};

struct Float_Root
{
    static constexpr auto whitespace = frst::grammar::ws;
    static constexpr auto rule =
        lexy::dsl::p<frst::grammar::float_literal> + lexy::dsl::eof;
    static constexpr auto value = lexy::forward<frst::Value_Ptr>;
};

struct Program_Root
{
    static constexpr auto rule = lexy::dsl::p<frst::grammar::program>;
    static constexpr auto value =
        lexy::forward<std::vector<frst::ast::Statement::Ptr>>;
};
} // namespace

TEST_CASE("Parser Numeric Literals")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse_int = [](std::string_view input) {
        auto src = lexy::string_input<lexy::utf8_encoding>(input);
        frst::grammar::reset_parse_state(src);
        return lexy::parse<Integer_Root>(src, lexy::noop);
    };
    auto parse_float = [](std::string_view input) {
        auto src = lexy::string_input<lexy::utf8_encoding>(input);
        frst::grammar::reset_parse_state(src);
        return lexy::parse<Float_Root>(src, lexy::noop);
    };

    SECTION("Valid integers")
    {
        struct Case
        {
            std::string_view input;
            frst::Int expected;
        };

        const Case cases[] = {
            {"0", 0_f},
            {"7", 7_f},
            {"42", 42_f},
            {"001", 1_f},
            {"9223372036854775807", 9223372036854775807_f},
            {"123456", 123456_f},
        };

        for (const auto& c : cases)
        {
            auto result = parse_int(c.input);
            REQUIRE(result);
            auto value = std::move(result).value();
            REQUIRE(value->is<frst::Int>());
            CHECK(value->get<frst::Int>().value() == c.expected);
        }
    }

    SECTION("Invalid integers")
    {
        const std::string_view cases[] = {
            "",
            "1.0",
            ".1",
            "1.",
            "1e3",
            "+1",
            "-1",
            "1_0",
            "12 34",
            "0x10",
            "9223372036854775808",
        };

        for (const auto& input : cases)
        {
            auto result = parse_int(input);
            CHECK_FALSE(result);
        }
    }

    SECTION("Valid floats")
    {
        struct Case
        {
            std::string_view input;
            frst::Float expected;
        };

        const Case cases[] = {
            {"0.0", 0.0},     {"1.0", 1.0},      {"3.14", 3.14},
            {"10.01", 10.01}, {"001.250", 1.25},
        };

        for (const auto& c : cases)
        {
            auto result = parse_float(c.input);
            REQUIRE(result);
            auto value = std::move(result).value();
            REQUIRE(value->is<frst::Float>());
            CHECK(value->get<frst::Float>().value() == c.expected);
        }
    }

    SECTION("Invalid floats")
    {
        // These inputs are never dispatched to float_literal by the full
        // parser (the Literal peek guards ensure only valid prefixes enter).
        // We test the subset that float_literal itself rejects.
        const std::string_view cases[] = {
            "",    "1",    "1.",    ".1",   "1.2.3",
            "+1.0", "-1.0", "1_0.2", "12 34",
        };

        for (const auto& input : cases)
        {
            DYNAMIC_SECTION("input: '" << input << "'")
            {
                auto result = parse_float(input);
                CHECK_FALSE(result);
            }
        }
    }

    SECTION("Scientific notation (via full parser)")
    {
        struct Case
        {
            std::string_view input;
            frst::Float expected;
        };

        const Case cases[] = {
            // decimal + signed exponent
            {"3.14e+10", 3.14e+10},
            {"1.0e-5", 1.0e-5},
            {"1.23e-10", 1.23e-10},
            {"9.99e+100", 9.99e+100},
            // decimal + unsigned exponent
            {"3.14e10", 3.14e10},
            {"1.0e2", 1.0e2},
            // integer + signed exponent
            {"1e+20", 1e+20},
            {"1e-05", 1e-05},
            {"5e+0", 5e+0},
            // integer + unsigned exponent
            {"3e2", 3e2},
            {"1e3", 1e3},
            {"5e0", 5e0},
        };

        auto parse_program = [](std::string_view input) {
            auto src = lexy::string_input<lexy::utf8_encoding>(input);
            frst::grammar::reset_parse_state(src);
            return lexy::parse<Program_Root>(src, lexy::noop);
        };

        for (const auto& c : cases)
        {
            DYNAMIC_SECTION(c.input)
            {
                auto result = parse_program(c.input);
                REQUIRE(result);
                auto program = std::move(result).value();
                REQUIRE(program.size() == 1);

                frst::Symbol_Table table;
                frst::Evaluation_Context ctx{.symbols = table};
                auto* expr = dynamic_cast<const frst::ast::Expression*>(
                    program[0].get());
                REQUIRE(expr);
                auto val = expr->evaluate(ctx);
                REQUIRE(val->is<frst::Float>());
                CHECK(val->get<frst::Float>().value() == c.expected);
            }
        }
    }

    SECTION("Scientific notation is Float, not Int")
    {
        auto parse_program = [](std::string_view input) {
            auto src = lexy::string_input<lexy::utf8_encoding>(input);
            frst::grammar::reset_parse_state(src);
            return lexy::parse<Program_Root>(src, lexy::noop);
        };

        // 3e2 should be Float(300), not Int(300)
        auto result = parse_program("3e2");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto* expr =
            dynamic_cast<const frst::ast::Expression*>(program[0].get());
        REQUIRE(expr);
        auto val = expr->evaluate(ctx);
        CHECK(val->is<frst::Float>());
        CHECK_FALSE(val->is<frst::Int>());
    }

    SECTION("Identifiers starting with 'e' are not floats")
    {
        auto parse_program = [](std::string_view input) {
            auto src = lexy::string_input<lexy::utf8_encoding>(input);
            frst::grammar::reset_parse_state(src);
            return lexy::parse<Program_Root>(src, lexy::noop);
        };

        // "e2" alone is an identifier, not a float
        auto result = parse_program("e2");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);
        // It should parse as a name lookup, not a literal
        auto* lookup = dynamic_cast<const frst::ast::Name_Lookup*>(
            program[0].get());
        CHECK(lookup != nullptr);
    }

    SECTION("Out of range floats")
    {
        auto too_large = std::string(400, '9') + ".0";
        CHECK_THROWS_WITH(parse_float(too_large),
                          Catch::Matchers::ContainsSubstring("out of range"));
    }
}
