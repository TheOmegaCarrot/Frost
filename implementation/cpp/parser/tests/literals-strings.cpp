#include <catch2/catch_test_macros.hpp>

#include <frost/testing/stringmaker-specializations.hpp>
#include <frost/value.hpp>

#include <lexy/action/parse.hpp>
#include <lexy/callback.hpp>
#include <lexy/input/string_input.hpp>

#include "../grammar.hpp"

namespace
{
struct String_Root
{
    static constexpr auto whitespace = frst::grammar::ws;
    static constexpr auto rule =
        lexy::dsl::p<frst::grammar::string_literal> + lexy::dsl::eof;
    static constexpr auto value = lexy::forward<frst::Value_Ptr>;
};
} // namespace

TEST_CASE("Parser String Literals")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        auto src = lexy::string_input(input);
        return lexy::parse<String_Root>(src, lexy::noop);
    };

    SECTION("Valid strings")
    {
        struct Case
        {
            std::string double_input;
            std::string single_input;
            std::string double_expected;
            std::string single_expected;
        };

        const Case cases[] = {
            {"\"\"", "''", "", ""},
            {"\"hello\"", "'hello'", "hello", "hello"},
            {"\"a b c\"", "'a b c'", "a b c", "a b c"},
            {"\"line1\\nline2\"", "'line1\\nline2'", "line1\nline2",
             "line1\nline2"},
            {"\"tab\\tsep\"", "'tab\\tsep'", "tab\tsep", "tab\tsep"},
            {"\"carriage\\rreturn\"", "'carriage\\rreturn'", "carriage\rreturn",
             "carriage\rreturn"},
            {"\"quote: \\\"\"", "'quote: \\''", "quote: \"", "quote: '"},
            {"\"backslash: \\\\\"", "'backslash: \\\\'", "backslash: \\",
             "backslash: \\"},
        };

        for (const auto& c : cases)
        {
            auto result = parse(c.double_input);
            REQUIRE(result);
            auto value = std::move(result).value();
            REQUIRE(value->is<frst::String>());
            CHECK(value->get<frst::String>().value() == c.double_expected);

            auto result2 = parse(c.single_input);
            REQUIRE(result2);
            auto value2 = std::move(result2).value();
            REQUIRE(value2->is<frst::String>());
            CHECK(value2->get<frst::String>().value() == c.single_expected);
        }
    }

    SECTION("Valid raw strings")
    {
        struct Case
        {
            std::string input;
            std::string expected;
        };

        const Case cases[] = {
            {"R\"()\"", ""},
            {"R'()'", ""},
            {"R\"(hello)\"", "hello"},
            {"R'(hello)'", "hello"},
            {"R\"(He said \"hi\")\"", "He said \"hi\""},
            {"R'(it's ok)'", "it's ok"},
            {"R\"(backslash \\\\ and \\n)\"", "backslash \\\\ and \\n"},
        };

        for (const auto& c : cases)
        {
            auto result = parse(c.input);
            REQUIRE(result);
            auto value = std::move(result).value();
            REQUIRE(value->is<frst::String>());
            CHECK(value->get<frst::String>().value() == c.expected);
        }
    }

    SECTION("Invalid strings")
    {
        struct Case
        {
            std::string double_input;
            std::string single_input;
        };

        const Case cases[] = {
            {"\"unterminated", "'unterminated"},
            {"\"bad\\qescape\"", "'bad\\qescape'"},
            {"\"bad\\xescape\"", "'bad\\xescape'"},
            {"\"bad\\u1234\"", "'bad\\u1234'"},
        };

        for (const auto& c : cases)
        {
            auto result = parse(c.double_input);
            CHECK_FALSE(result);

            auto result2 = parse(c.single_input);
            CHECK_FALSE(result2);
        }
    }

    SECTION("Invalid raw strings")
    {
        const std::string cases[] = {
            "R\"(unterminated\"",
            "R'(unterminated'",
            "R\"(mismatch)'",
            "R'(mismatch)\"",
        };

        for (const auto& input : cases)
        {
            auto result = parse(input);
            CHECK_FALSE(result);
        }
    }

    SECTION("Newlines are not allowed in strings")
    {
        std::string double_input = "\"line1\nline2\"";
        auto result = parse(double_input);
        CHECK_FALSE(result);

        std::string single_input = "'line1\nline2'";
        auto result2 = parse(single_input);
        CHECK_FALSE(result2);
    }

    SECTION("Newlines are not allowed in raw strings")
    {
        std::string raw_double = "R\"(line1\nline2)\"";
        auto result = parse(raw_double);
        CHECK_FALSE(result);

        std::string raw_single = "R'(line1\nline2)'";
        auto result2 = parse(raw_single);
        CHECK_FALSE(result2);
    }
}
