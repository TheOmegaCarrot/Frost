#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/ast.hpp>
#include <frost/parser.hpp>
#include <frost/symbol-table.hpp>
#include <frost/testing/stringmaker-specializations.hpp>
#include <frost/value.hpp>

using namespace frst::literals;

TEST_CASE("Parser Primary Expressions")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        return frst::parse_data(std::string{input});
    };

    SECTION("Literals evaluate correctly")
    {
        struct Int_Case
        {
            std::string_view input;
            frst::Int expected;
        };

        const Int_Case int_cases[] = {
            {"0", 0_f},
            {"42", 42_f},
        };

        for (const auto& c : int_cases)
        {
            auto result = parse(c.input);
            REQUIRE(result.has_value());
            auto expr = std::move(result).value();

            frst::Symbol_Table table;
            frst::Evaluation_Context ctx{.symbols = table};
            auto value = expr->evaluate(ctx);
            REQUIRE(value->is<frst::Int>());
            CHECK(value->get<frst::Int>().value() == c.expected);
        }

        struct Float_Case
        {
            std::string_view input;
            frst::Float expected;
        };

        const Float_Case float_cases[] = {
            {"1.5", 1.5},
            {"10.25", 10.25},
        };

        for (const auto& c : float_cases)
        {
            auto result = parse(c.input);
            REQUIRE(result.has_value());
            auto expr = std::move(result).value();

            frst::Symbol_Table table;
            frst::Evaluation_Context ctx{.symbols = table};
            auto value = expr->evaluate(ctx);
            REQUIRE(value->is<frst::Float>());
            CHECK(value->get<frst::Float>().value() == c.expected);
        }

        struct String_Case
        {
            std::string_view input;
            std::string_view expected;
        };

        const String_Case string_cases[] = {
            {"\"hi\"", "hi"},
            {"\"a b\"", "a b"},
        };

        for (const auto& c : string_cases)
        {
            auto result = parse(c.input);
            REQUIRE(result.has_value());
            auto expr = std::move(result).value();

            frst::Symbol_Table table;
            frst::Evaluation_Context ctx{.symbols = table};
            auto value = expr->evaluate(ctx);
            REQUIRE(value->is<frst::String>());
            CHECK(value->get<frst::String>().value() == c.expected);
        }

        const std::string_view bool_cases[] = {"true", "false"};
        const bool bool_expected[] = {true, false};

        for (std::size_t i = 0; i < std::size(bool_cases); ++i)
        {
            auto result = parse(bool_cases[i]);
            REQUIRE(result.has_value());
            auto expr = std::move(result).value();

            frst::Symbol_Table table;
            frst::Evaluation_Context ctx{.symbols = table};
            auto value = expr->evaluate(ctx);
            REQUIRE(value->is<frst::Bool>());
            CHECK(value->get<frst::Bool>().value() == bool_expected[i]);
        }

        auto null_result = parse("null");
        REQUIRE(null_result.has_value());
        auto null_expr = std::move(null_result).value();
        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto null_value = null_expr->evaluate(ctx);
        REQUIRE(null_value->is<frst::Null>());
    }

    SECTION("Name lookup evaluates against the symbol table")
    {
        auto result = parse("foo");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto value = frst::Value::create(42_f);
        table.define("foo", value);

        auto evaluated = expr->evaluate(ctx);
        CHECK(evaluated == value);
    }

    SECTION("Parenthesized expressions parse and evaluate")
    {
        auto int_result = parse("(  7 )");
        REQUIRE(int_result.has_value());
        auto int_expr = std::move(int_result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto int_value = int_expr->evaluate(ctx);
        REQUIRE(int_value->is<frst::Int>());
        CHECK(int_value->get<frst::Int>().value() == 7_f);

        auto nested_result = parse("((name))");
        REQUIRE(nested_result.has_value());
        auto nested_expr = std::move(nested_result).value();

        auto stored = frst::Value::create(123_f);
        table.define("name", stored);
        auto nested_value = nested_expr->evaluate(ctx);
        CHECK(nested_value == stored);

        auto comment_result = parse("(# comment\nname)");
        REQUIRE(comment_result.has_value());
        auto comment_expr = std::move(comment_result).value();
        auto comment_value = comment_expr->evaluate(ctx);
        CHECK(comment_value == stored);
    }

    SECTION("Invalid parenthesized expressions fail to parse")
    {
        const std::string_view cases[] = {
            "()", "(", ")", "(1", "1)", "((1)", "(1))",
        };

        for (const auto& input : cases)
        {
            CHECK(not parse(input));
        }
    }

    SECTION("Name lookup failure propagates as an error")
    {
        auto result = parse("missing");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        CHECK_THROWS_WITH(expr->evaluate(ctx),
                          Catch::Matchers::ContainsSubstring("not defined"));
        CHECK_THROWS_WITH(expr->evaluate(ctx),
                          Catch::Matchers::ContainsSubstring("missing"));
    }

    SECTION("Reserved keywords fail as expressions")
    {
        const std::string_view cases[] = {
            "def",    "if",     "else",    "elif", "map",
            "filter", "reduce", "foreach", "with", "init",
        };

        for (const auto& input : cases)
        {
            CHECK(not parse(input));
        }
    }

    SECTION("Expression parsing rejects extra tokens")
    {
        const std::string_view cases[] = {
            "(true)false",
            "name name",
            "null true",
            "\"hi\" \"there\"",
        };

        for (const auto& input : cases)
        {
            CHECK(not parse(input));
        }
    }

    SECTION("Parenthesized lookup failure propagates")
    {
        auto result = parse("(missing)");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        CHECK_THROWS_WITH(expr->evaluate(ctx),
                          Catch::Matchers::ContainsSubstring("not defined"));
        CHECK_THROWS_WITH(expr->evaluate(ctx),
                          Catch::Matchers::ContainsSubstring("missing"));
    }

    SECTION("Keyword prefixes parse as identifiers")
    {
        const std::string_view cases[] = {"truex", "null_", "if1"};

        for (const auto& input : cases)
        {
            auto result = parse(input);
            REQUIRE(result.has_value());
            auto expr = std::move(result).value();

            frst::Symbol_Table table;
            frst::Evaluation_Context ctx{.symbols = table};
            auto value = frst::Value::create(1_f);
            table.define(std::string{input}, value);
            auto evaluated = expr->evaluate(ctx);
            CHECK(evaluated == value);
        }
    }

    SECTION("Null literal returns the singleton")
    {
        auto result = parse("null");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto value = expr->evaluate(ctx);
        CHECK(value == frst::Value::null());
    }

    SECTION("Source range for do block")
    {
        // "do { 1 }" → [1:1-1:8]
        auto result = parse("do { 1 }");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        auto range = expr->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 1);
        CHECK(range.end.line == 1);
        CHECK(range.end.column == 8);
    }

    SECTION("Source range for multiline do block")
    {
        // "do {\n  1\n}" → [1:1-3:1]
        auto result = parse("do {\n  1\n}");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        auto range = expr->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 1);
        CHECK(range.end.line == 3);
        CHECK(range.end.column == 1);
    }

    SECTION("Source range for do block excludes trailing whitespace")
    {
        auto result = parse("do { 1 }   ");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        auto range = expr->source_range();
        CHECK(range.begin.column == 1);
        CHECK(range.end.column == 8);
    }

    SECTION("Source range for keyword literals")
    {
        // "true" → [1:1-1:4]
        auto r_true = parse("true");
        REQUIRE(r_true.has_value());
        auto e_true = std::move(r_true).value();
        CHECK(e_true->source_range().begin.column == 1);
        CHECK(e_true->source_range().end.column == 4);

        // "false" → [1:1-1:5]
        auto r_false = parse("false");
        REQUIRE(r_false.has_value());
        auto e_false = std::move(r_false).value();
        CHECK(e_false->source_range().begin.column == 1);
        CHECK(e_false->source_range().end.column == 5);

        // "null" → [1:1-1:4]
        auto r_null = parse("null");
        REQUIRE(r_null.has_value());
        auto e_null = std::move(r_null).value();
        CHECK(e_null->source_range().begin.column == 1);
        CHECK(e_null->source_range().end.column == 4);
    }

    SECTION("Source range for multi-digit integer literal")
    {
        // "12345" → [1:1-1:5]
        auto result = parse("12345");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        CHECK(expr->source_range().begin.column == 1);
        CHECK(expr->source_range().end.column == 5);
    }

    SECTION("Source range for float literal")
    {
        // "3.14" → [1:1-1:4]
        auto result = parse("3.14");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        CHECK(expr->source_range().begin.column == 1);
        CHECK(expr->source_range().end.column == 4);
    }

    SECTION("Source range for string literals")
    {
        // "'hello'" → [1:1-1:7]
        auto r_single = parse("'hello'");
        REQUIRE(r_single.has_value());
        auto e_single = std::move(r_single).value();
        CHECK(e_single->source_range().begin.column == 1);
        CHECK(e_single->source_range().end.column == 7);

        // R"("world")" → [1:1-1:7]
        auto r_double = parse("\"world\"");
        REQUIRE(r_double.has_value());
        auto e_double = std::move(r_double).value();
        CHECK(e_double->source_range().begin.column == 1);
        CHECK(e_double->source_range().end.column == 7);
    }

    SECTION("Source range for empty string literal")
    {
        // "''" → [1:1-1:2]
        auto result = parse("''");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        CHECK(expr->source_range().begin.column == 1);
        CHECK(expr->source_range().end.column == 2);
    }
}
