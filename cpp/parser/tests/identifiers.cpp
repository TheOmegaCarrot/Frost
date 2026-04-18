#include <catch2/catch_test_macros.hpp>

#include <frost/ast.hpp>
#include <frost/parser.hpp>
#include <frost/testing/stringmaker-specializations.hpp>

TEST_CASE("Parser Identifiers")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        return frst::parse_data(std::string{input});
    };

    SECTION("Valid identifiers")
    {
        const std::string_view cases[] = {
            "a",      "A",    "_foo",    "foo_",  "foo_bar",
            "foo123", "a1_b2", "if1", "trueish", "null_",
        };

        for (const auto& input : cases)
        {
            auto result = parse(input);
            REQUIRE(result.has_value());
            auto expr = std::move(result).value();
            auto* lookup =
                dynamic_cast<const frst::ast::Name_Lookup*>(expr.get());
            REQUIRE(lookup);
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
            // Either parse fails, or it produces something that is not a
            // Name_Lookup (e.g. a literal for "true"/"false"/"null")
            if (result.has_value())
            {
                auto* lookup = dynamic_cast<const frst::ast::Name_Lookup*>(
                    result.value().get());
                CHECK_FALSE(lookup);
            }
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

        for (const auto& input : cases)
        {
            auto result = parse(input);
            REQUIRE(result.has_value());
            auto expr = std::move(result).value();
            auto* lookup =
                dynamic_cast<const frst::ast::Name_Lookup*>(expr.get());
            REQUIRE(lookup);
        }
    }
}
