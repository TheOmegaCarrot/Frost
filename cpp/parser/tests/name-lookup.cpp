#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/ast.hpp>
#include <frost/parser.hpp>
#include <frost/symbol-table.hpp>
#include <frost/testing/stringmaker-specializations.hpp>

using namespace frst::literals;

TEST_CASE("Name Lookup")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        return frst::parse_data(std::string{input});
    };

    SECTION("Valid identifiers")
    {
        struct Case
        {
            std::string_view input;
            std::string_view expected;
        };

        const Case cases[] = {
            {"foo", "foo"}, {"_bar", "_bar"}, {"a1_b2", "a1_b2"},
            {"_a", "_a"},   {"if1", "if1"},
        };

        for (const auto& c : cases)
        {
            auto result = parse(c.input);
            REQUIRE(result.has_value());
            auto expr = std::move(result).value();

            frst::Symbol_Table table;
            frst::Evaluation_Context ctx{.symbols = table};
            auto value = frst::Value::create(42_f);
            table.define(std::string{c.expected}, value);

            auto evaluated = expr->evaluate(ctx);
            CHECK(evaluated == value);
        }
    }

    SECTION("Invalid identifiers fail to parse")
    {
        const std::string_view cases[] = {
            "", "1abc", "-abc", "a-b", "a b", "if", "else", "true", "null",
        };

        for (const auto& input : cases)
        {
            auto result = parse(input);
            // Either parse fails, or it produces something other than a
            // Name_Lookup (e.g. "true"/"null" are valid literals, not lookups)
            if (result.has_value())
            {
                auto* lookup = dynamic_cast<const frst::ast::Name_Lookup*>(
                    result.value().get());
                CHECK_FALSE(lookup);
            }
        }
    }

    SECTION("Lookup fails when identifier is not defined")
    {
        auto result = parse("missing_name");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        CHECK_THROWS_WITH(expr->evaluate(ctx),
                          Catch::Matchers::ContainsSubstring("not defined"));
        CHECK_THROWS_WITH(expr->evaluate(ctx),
                          Catch::Matchers::ContainsSubstring("missing_name"));
    }
}

TEST_CASE("Name Lookup Source Ranges")
{
    auto parse = [](std::string_view input) {
        return frst::parse_data(std::string{input});
    };

    SECTION("Single-character identifier")
    {
        // "x" → [1:1-1:1]
        auto result = parse("x");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        REQUIRE(expr);
        auto range = expr->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 1);
        CHECK(range.end.column == 1);
    }

    SECTION("Multi-character identifier")
    {
        // "foobar" → [1:1-1:6]
        auto result = parse("foobar");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        REQUIRE(expr);
        auto range = expr->source_range();
        CHECK(range.begin.column == 1);
        CHECK(range.end.column == 6);
    }

    SECTION("Identifier with underscores")
    {
        // "my_var_2" → [1:1-1:8]
        auto result = parse("my_var_2");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        REQUIRE(expr);
        auto range = expr->source_range();
        CHECK(range.begin.column == 1);
        CHECK(range.end.column == 8);
    }
}
