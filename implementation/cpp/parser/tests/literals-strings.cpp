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
            std::string input;
            std::string expected;
        };

        const Case cases[] = {
            {"\"\"", ""},
            {"\"hello\"", "hello"},
            {"\"a b c\"", "a b c"},
            {"\"line1\\nline2\"", "line1\nline2"},
            {"\"tab\\tsep\"", "tab\tsep"},
            {"\"carriage\\rreturn\"", "carriage\rreturn"},
            {"\"quote: \\\"\"", "quote: \""},
            {"\"backslash: \\\\\"", "backslash: \\"},
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
        const std::string cases[] = {
            "\"unterminated",
            "\"bad\\qescape\"",
            "\"bad\\xescape\"",
            "\"bad\\u1234\"",
        };

        for (const auto& input : cases)
        {
            auto result = parse(input);
            CHECK_FALSE(result);
        }
    }

    SECTION("Newlines are not allowed in strings")
    {
        std::string input = "\"line1\nline2\"";
        auto result = parse(input);
        CHECK_FALSE(result);
    }
}
