#include <catch2/catch_test_macros.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <lexy/action/parse.hpp>
#include <lexy/callback.hpp>
#include <lexy/input/string_input.hpp>

#include "../grammar.hpp"

namespace
{
struct Identifier_Root
{
    static constexpr auto whitespace = frst::grammar::ws;
    static constexpr auto rule =
        lexy::dsl::p<frst::grammar::identifier> + lexy::dsl::eof;
    static constexpr auto value = lexy::forward<std::string>;
};
} // namespace

TEST_CASE("Parser Identifiers")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        auto src = lexy::string_input(input);
        return lexy::parse<Identifier_Root>(src, lexy::noop);
    };

    SECTION("Valid identifiers")
    {
        const std::string_view cases[] = {
            "a",      "A",     "_",   "_foo",    "foo_",  "foo_bar",
            "foo123", "a1_b2", "if1", "trueish", "null_",
        };

        for (const auto& input : cases)
        {
            auto result = parse(input);
            REQUIRE(result);
            CHECK(std::move(result).value() == input);
        }
    }

    SECTION("Invalid identifiers")
    {
        const std::string_view cases[] = {
            "",       "1abc", "9",       "-abc",   "a-b",  "a b",  "a.b",
            "a@b",    "if",   "else",    "elif",   "def",  "fn",   "reduce",
            "export", "map",  "foreach", "filter", "with", "init", "true",
            "false",  "and",  "or",      "not",    "null",
        };

        for (const auto& input : cases)
        {
            auto result = parse(input);
            CHECK_FALSE(result);
            CHECK(result.error_count() >= 1);
        }
    }

    SECTION("Comments and whitespace are allowed around identifiers")
    {
        const std::string_view cases[] = {
            "  foo  ",
            "\n\tbar\n",
            "baz # trailing comment",
            "# full line comment\nqux",
        };

        const std::string_view expected[] = {"foo", "bar", "baz", "qux"};

        for (std::size_t i = 0; i < std::size(cases); ++i)
        {
            auto result = parse(cases[i]);
            REQUIRE(result);
            CHECK(std::move(result).value() == expected[i]);
        }
    }
}
