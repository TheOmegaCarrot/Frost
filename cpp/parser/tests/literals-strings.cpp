#include <catch2/catch_test_macros.hpp>

#include <expected>

#include <frost/ast.hpp>
#include <frost/parser.hpp>
#include <frost/symbol-table.hpp>
#include <frost/testing/stringmaker-specializations.hpp>
#include <frost/value.hpp>

TEST_CASE("Parser String Literals")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        return frst::parse_data(std::string{input});
    };

    auto eval_string =
        [](std::expected<frst::ast::Expression::Ptr, std::string>& result)
        -> std::string {
        auto expr = std::move(result).value();
        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto val = expr->evaluate(ctx);
        REQUIRE(val->is<frst::String>());
        return std::string{val->get<frst::String>().value()};
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
            REQUIRE(result.has_value());
            CHECK(eval_string(result) == c.double_expected);

            auto result2 = parse(c.single_input);
            REQUIRE(result2.has_value());
            CHECK(eval_string(result2) == c.single_expected);
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
            REQUIRE(result.has_value());
            CHECK(eval_string(result) == c.expected);
        }
    }

    SECTION("UTF-8 literals are allowed in string literals")
    {
        const std::string emoji = "😀";
        const auto expect_roundtrip = [&](const std::string& input) {
            auto result = parse(input);
            REQUIRE(result.has_value());
            CHECK(eval_string(result) == emoji);
        };

        expect_roundtrip("\"" + emoji + "\"");
        expect_roundtrip("'" + emoji + "'");
        expect_roundtrip("R\"(" + emoji + ")\"");
    }

    SECTION("Hex escape sequences")
    {
        using namespace std::literals;

        struct Case
        {
            std::string double_input;
            std::string single_input;
            std::string expected;
        };

        const Case cases[] = {
            // Basic byte values
            {"\"\\x41\"", "'\\x41'", "A"},
            {"\"\\x00\"", "'\\x00'", "\0"s},
            {"\"\\x0a\"", "'\\x0a'", "\n"},
            {"\"\\x09\"", "'\\x09'", "\t"},
            {"\"\\x7f\"", "'\\x7f'", "\x7f"s},
            {"\"\\x80\"", "'\\x80'", "\x80"s},
            {"\"\\xff\"", "'\\xff'", "\xff"s},
            // Uppercase hex digits
            {"\"\\xFF\"", "'\\xFF'", "\xff"s},
            {"\"\\x0A\"", "'\\x0A'", "\n"},
            {"\"\\xAB\"", "'\\xAB'", "\xab"s},
            // Mixed case
            {"\"\\xaF\"", "'\\xaF'", "\xaf"s},
            // Multiple hex escapes
            {"\"\\x48\\x69\"", "'\\x48\\x69'", "Hi"},
            // Mixed with named escapes
            {"\"\\x41\\n\\x42\"", "'\\x41\\n\\x42'", "A\nB"},
            {"\"\\t\\x20\\\\\"", "'\\t\\x20\\\\'", "\t \\"},
            // Adjacent to plain text
            {"\"abc\\x21def\"", "'abc\\x21def'", "abc!def"},
        };

        for (const auto& c : cases)
        {
            INFO("double: " << c.double_input);
            auto result = parse(c.double_input);
            REQUIRE(result.has_value());
            CHECK(eval_string(result) == c.expected);

            INFO("single: " << c.single_input);
            auto result2 = parse(c.single_input);
            REQUIRE(result2.has_value());
            CHECK(eval_string(result2) == c.expected);
        }
    }

    SECTION("Hex escapes are not processed in raw strings")
    {
        auto result = parse("R\"(\\x41)\"");
        REQUIRE(result.has_value());
        CHECK(eval_string(result) == "\\x41");

        auto result2 = parse("R'(\\x41)'");
        REQUIRE(result2.has_value());
        CHECK(eval_string(result2) == "\\x41");
    }

    SECTION("Invalid hex escapes")
    {
        // Only one hex digit
        CHECK(not parse("\"\\x0\""));
        CHECK(not parse("'\\x0'"));

        // Non-hex digit after \x
        CHECK(not parse("\"\\xGG\""));
        CHECK(not parse("'\\xGG'"));

        // \x at end of string (no digits)
        CHECK(not parse("\"\\x\""));
        CHECK(not parse("'\\x'"));
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
            {"\"bad\\u1234\"", "'bad\\u1234'"},
        };

        for (const auto& c : cases)
        {
            CHECK(not parse(c.double_input));
            CHECK(not parse(c.single_input));
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
            CHECK(not parse(input));
        }
    }

    SECTION("Newlines are not allowed in strings")
    {
        std::string double_input = "\"line1\nline2\"";
        CHECK(not parse(double_input));

        std::string single_input = "'line1\nline2'";
        CHECK(not parse(single_input));
    }

    SECTION("Newlines are not allowed in raw strings")
    {
        std::string raw_double = "R\"(line1\nline2)\"";
        CHECK(not parse(raw_double));

        std::string raw_single = "R'(line1\nline2)'";
        CHECK(not parse(raw_single));
    }

    SECTION("Triple-quoted strings: basic multiline")
    {
        auto result = parse(R"("""
    hello
    world
    """)");
        REQUIRE(result.has_value());
        CHECK(eval_string(result) == R"(hello
world)");
    }

    SECTION("Triple-quoted strings: single-quoted variant")
    {
        auto result = parse(R"('''
    hello
    world
    ''')");
        REQUIRE(result.has_value());
        CHECK(eval_string(result) == R"(hello
world)");
    }

    SECTION("Triple-quoted strings: indentation beyond closing is preserved")
    {
        auto result = parse(R"("""
    if true:
      nested
    """)");
        REQUIRE(result.has_value());
        CHECK(eval_string(result) == R"(if true:
  nested)");
    }

    SECTION("Triple-quoted strings: empty lines preserved")
    {
        auto result = parse(R"("""
    line 1

    line 3
    """)");
        REQUIRE(result.has_value());
        CHECK(eval_string(result) == R"(line 1

line 3)");
    }

    SECTION("Triple-quoted strings: \\t escape")
    {
        auto result = parse(R"("""
    hello\tworld
    """)");
        REQUIRE(result.has_value());
        CHECK(eval_string(result) == "hello\tworld");
    }

    SECTION("Triple-quoted strings: \\\\ escape")
    {
        auto result = parse(R"("""
    back\\slash
    """)");
        REQUIRE(result.has_value());
        CHECK(eval_string(result) == "back\\slash");
    }

    SECTION("Triple-quoted strings: \\0 escape")
    {
        using namespace std::literals;
        auto result = parse(R"("""
    null\0byte
    """)");
        REQUIRE(result.has_value());
        CHECK(eval_string(result) == "null\0byte"s);
    }

    SECTION("Triple-quoted strings: escaped delimiter quotes")
    {
        auto result = parse(R"("""
    say \"hello\"
    """)");
        REQUIRE(result.has_value());
        CHECK(eval_string(result) == R"(say "hello")");

        auto single = parse(R"('''
    it\'s escaped
    ''')");
        REQUIRE(single.has_value());
        CHECK(eval_string(single) == "it's escaped");
    }

    SECTION("Triple-quoted strings: hex escapes are forbidden")
    {
        auto result = parse(R"("""
    \x48\x69
    """)");
        CHECK(not result);
    }

    SECTION("Triple-quoted strings: no indentation on closing delimiter")
    {
        auto result = parse(R"("""
hello
world
""")");
        REQUIRE(result.has_value());
        CHECK(eval_string(result) == R"(hello
world)");
    }

    SECTION("Triple-quoted strings: single content line")
    {
        auto result = parse(R"("""
    hello
    """)");
        REQUIRE(result.has_value());
        CHECK(eval_string(result) == "hello");
    }

    SECTION("Triple-quoted strings: can contain double quotes")
    {
        auto result = parse(R"("""
    she said "hi"
    """)");
        REQUIRE(result.has_value());
        CHECK(eval_string(result) == R"(she said "hi")");
    }

    SECTION("Triple-quoted strings: single-quoted can contain single quotes")
    {
        auto result = parse(R"('''
    it's fine
    ''')");
        REQUIRE(result.has_value());
        CHECK(eval_string(result) == "it's fine");
    }

    SECTION("Triple-quoted strings: triple-single can contain triple-double")
    {
        auto result = parse(R"--('''
    some """quoted""" text
    ''')--");
        REQUIRE(result.has_value());
        CHECK(eval_string(result) == R"(some """quoted""" text)");
    }

    SECTION("Triple-quoted strings: triple-double can contain triple-single")
    {
        auto result = parse(R"("""
    some '''quoted''' text
    """)");
        REQUIRE(result.has_value());
        CHECK(eval_string(result) == "some '''quoted''' text");
    }

    SECTION("Triple-quoted strings: tab indentation")
    {
        auto result = parse("\"\"\"\n\thello\n\tworld\n\t\"\"\"");
        REQUIRE(result.has_value());
        CHECK(eval_string(result) == R"(hello
world)");
    }

    SECTION("Triple-quoted strings: mixed indentation levels")
    {
        auto result = parse(R"("""
    a
        b
            c
    """)");
        REQUIRE(result.has_value());
        CHECK(eval_string(result) == R"(a
    b
        c)");
    }

    SECTION("Triple-quoted strings: empty content")
    {
        auto result = parse("\"\"\"\n\"\"\"");
        REQUIRE(result.has_value());
        CHECK(eval_string(result) == "");
    }

    SECTION("Triple-quoted strings: whitespace-only content")
    {
        auto result = parse(R"("""

    """)");
        REQUIRE(result.has_value());
        CHECK(eval_string(result) == "");
    }

    SECTION("Triple-quoted strings: \\n and \\r escapes are forbidden")
    {
        auto with_n = parse(R"("""
    hello\nworld
    """)");
        CHECK(not with_n);

        auto with_r = parse(R"("""
    hello\rworld
    """)");
        CHECK(not with_r);
    }

    SECTION("Triple-quoted strings: regular strings still work")
    {
        auto regular = parse(R"("hello")");
        REQUIRE(regular.has_value());
        CHECK(eval_string(regular) == "hello");

        auto single = parse("'hello'");
        REQUIRE(single.has_value());
        CHECK(eval_string(single) == "hello");
    }
}
