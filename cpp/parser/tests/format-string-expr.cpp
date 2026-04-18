#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_exception.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/ast.hpp>
#include <frost/parser.hpp>
#include <frost/symbol-table.hpp>
#include <frost/testing/stringmaker-specializations.hpp>
#include <frost/value.hpp>

using namespace frst::literals;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;
using Catch::Matchers::MessageMatches;

TEST_CASE("Parser Format Strings")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        return frst::parse_data(std::string{input});
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
            REQUIRE(result.has_value());
            auto expr = std::move(result).value();

            frst::Symbol_Table table;
            frst::Evaluation_Context ctx{.symbols = table};
            auto value = expr->evaluate(ctx);
            REQUIRE(value->is<frst::String>());
            CHECK(value->get<frst::String>().value() == "hello");
        }
    }

    SECTION("Placeholders evaluate against the symbol table")
    {
        auto result = parse(R"($"value ${x}")");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        table.define("x", frst::Value::create(42_f));

        auto value = expr->evaluate(ctx);
        REQUIRE(value->is<frst::String>());
        CHECK(value->get<frst::String>().value() == "value 42");
    }

    SECTION("Hex escapes work in format strings")
    {
        auto result = parse(R"($"\x48\x69 ${name}\x21")");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        table.define("name", frst::Value::create("world"s));

        auto value = expr->evaluate(ctx);
        REQUIRE(value->is<frst::String>());
        CHECK(value->get<frst::String>().value() == "Hi world!");
    }

    SECTION("Escaped placeholder is treated as literal")
    {
        auto result = parse(R"($"\\${x}")");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        table.define("x", frst::Value::create(1_f));

        auto value = expr->evaluate(ctx);
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
            CHECK(not parse(input));
        }
    }

    SECTION("Whitespace between $ and quote is not permitted")
    {
        CHECK(not parse(R"($ "hello")"));
    }

    SECTION("Non-string tokens after $ are rejected")
    {
        CHECK(not parse("$42"));
    }

    SECTION("Adjacent expressions after a format string are rejected")
    {
        CHECK(not parse(R"($"x"y)"));
    }

    SECTION("Malformed placeholders are rejected")
    {
        CHECK(not parse(R"($"${}")"));
        CHECK(not parse(R"($"${1}")"));
        CHECK(not parse(R"($"${foo${bar}}")"));
    }

    SECTION("Missing name lookup propagates as an error")
    {
        auto result = parse(R"($"${missing}")");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        CHECK_THROWS_MATCHES(expr->evaluate(ctx),
                             frst::Frost_Unrecoverable_Error,
                             MessageMatches(ContainsSubstring("missing")));
    }

    SECTION("Format string inside a lambda expression parses")
    {
        auto result = parse(R"(fn () -> { $"hello ${name}" })");
        CHECK(result.has_value());
    }

    SECTION("Format string in composite expressions parses")
    {
        const std::string_view cases[] = {
            R"([$"hi"])", R"({ key: $"hi" })", R"($"a" == $"b")",
            R"($"x"(1))", R"($"x"[0])",
        };

        for (const auto& input : cases)
        {
            CHECK(parse(input).has_value());
        }
    }

    SECTION("Source range for format string")
    {
        // "$'hello ${name}'" → [1:1-1:17]
        auto result = parse("$'hello ${name}'");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        auto range = expr->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 1);
        CHECK(range.end.line == 1);
        CHECK(range.end.column == 16);
    }

    SECTION("Source range for double-quoted format string")
    {
        // R"($"x=${val}")" → [1:1-1:11]
        auto result = parse(R"($"x=${val}")");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        auto range = expr->source_range();
        CHECK(range.begin.column == 1);
        CHECK(range.end.column == 11);
    }

    SECTION("Source range excludes trailing whitespace")
    {
        auto result = parse("$'hi'   ");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        auto range = expr->source_range();
        CHECK(range.begin.column == 1);
        CHECK(range.end.column == 5);
    }
}
