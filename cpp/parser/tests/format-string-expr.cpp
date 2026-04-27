#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_exception.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/ast.hpp>
#include <frost/builtin.hpp>
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

    auto parse_prog = [](std::string_view input) {
        return frst::parse_program(std::string{input});
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

    SECTION("Escaped dollar prevents interpolation")
    {
        // `\$` produces a literal `$`, preventing `${x}` from being
        // treated as an interpolation.
        auto result = parse(R"($"\${x}")");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};

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

    SECTION("Empty placeholder is rejected")
    {
        CHECK(not parse(R"($"${}")"));
    }

    SECTION("Expression placeholders are accepted")
    {
        // These were rejected by the old identifier-only format strings
        CHECK(parse(R"($"${1}")"));
        CHECK(parse(R"($"${1 + 2}")"));
        CHECK(parse(R"($"${fn x -> x}")"));
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

    // =================================================================
    // Expression interpolation: end-to-end evaluation
    // =================================================================

    SECTION("Arithmetic expression in interpolation")
    {
        auto result = parse(R"($"${1 + 2}")");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto value = expr->evaluate(ctx);
        REQUIRE(value->is<frst::String>());
        CHECK(value->get<frst::String>().value() == "3");
    }

    SECTION("Function call in interpolation")
    {
        auto result = parse_prog("def name = 'hello'\n"
                                 "def x = $\"${to_upper(name)}\"");
        REQUIRE(result.has_value());
        auto program = std::move(result).value();

        frst::Symbol_Table table;
        frst::inject_builtins(table);
        frst::Execution_Context ctx{.symbols = table};
        for (const auto& stmt : program)
            stmt->execute(ctx);
        CHECK(table.lookup("x")->raw_get<frst::String>() == "HELLO");
    }

    SECTION("Spaces preserved around interpolations")
    {
        auto result = parse_prog("def x = 1\n"
                                 "def s = $'a ${x} b'");
        REQUIRE(result.has_value());
        auto program = std::move(result).value();

        frst::Symbol_Table table;
        frst::Execution_Context ctx{.symbols = table};
        for (const auto& stmt : program)
            stmt->execute(ctx);
        CHECK(table.lookup("s")->raw_get<frst::String>() == "a 1 b");
    }

    SECTION("No spaces: chars directly adjacent to interpolation")
    {
        auto result = parse_prog("def x = 1\n"
                                 "def s = $'a${x}b'");
        REQUIRE(result.has_value());
        auto program = std::move(result).value();

        frst::Symbol_Table table;
        frst::Execution_Context ctx{.symbols = table};
        for (const auto& stmt : program)
            stmt->execute(ctx);
        CHECK(table.lookup("s")->raw_get<frst::String>() == "a1b");
    }

    SECTION("Map literal inside interpolation (balanced braces)")
    {
        auto result = parse(R"($"${{}}")");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto value = expr->evaluate(ctx);
        REQUIRE(value->is<frst::String>());
        CHECK(value->get<frst::String>().value() == "{}");
    }

    SECTION("Do block inside interpolation (balanced braces)")
    {
        auto result = parse(R"($"${do { def x = 42; x }}")");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto value = expr->evaluate(ctx);
        REQUIRE(value->is<frst::String>());
        CHECK(value->get<frst::String>().value() == "42");
    }

    SECTION("Closure captures through format string interpolation")
    {
        auto result = parse_prog("def make = fn name -> $'hello ${name}'\n"
                                 "def x = make('world')");
        REQUIRE(result.has_value());
        auto program = std::move(result).value();

        frst::Symbol_Table table;
        frst::Execution_Context ctx{.symbols = table};
        for (const auto& stmt : program)
            stmt->execute(ctx);
        CHECK(table.lookup("x")->raw_get<frst::String>() == "hello world");
    }

    SECTION("Escaped dollar not followed by brace")
    {
        auto result = parse(R"($"price \$5")");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto value = expr->evaluate(ctx);
        REQUIRE(value->is<frst::String>());
        CHECK(value->get<frst::String>().value() == "price $5");
    }

    SECTION("Multiple expressions with mixed types")
    {
        auto result = parse_prog("def n = 42\n"
                                 "def b = true\n"
                                 "def s = $'n=${n}, b=${b}, null=${null}'");
        REQUIRE(result.has_value());
        auto program = std::move(result).value();

        frst::Symbol_Table table;
        frst::Execution_Context ctx{.symbols = table};
        for (const auto& stmt : program)
            stmt->execute(ctx);
        CHECK(table.lookup("s")->raw_get<frst::String>()
              == "n=42, b=true, null=null");
    }

    SECTION("Conditional expression inside interpolation")
    {
        auto result = parse(R"($"${if true: 'yes' else: 'no'}")");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto value = expr->evaluate(ctx);
        REQUIRE(value->is<frst::String>());
        CHECK(value->get<frst::String>().value() == "yes");
    }

    SECTION("Empty format string")
    {
        for (auto input : {R"($"")", R"($'')"})
        {
            auto result = parse(input);
            REQUIRE(result.has_value());
            auto expr = std::move(result).value();

            frst::Symbol_Table table;
            frst::Evaluation_Context ctx{.symbols = table};
            auto value = expr->evaluate(ctx);
            REQUIRE(value->is<frst::String>());
            CHECK(value->get<frst::String>().value().empty());
        }
    }

    SECTION("Bare dollar in format string is literal")
    {
        auto result = parse(R"($"cost: $5")");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto value = expr->evaluate(ctx);
        REQUIRE(value->is<frst::String>());
        CHECK(value->get<frst::String>().value() == "cost: $5");
    }
}
