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
        auto src = lexy::string_input<lexy::utf8_encoding>(input);
        frst::grammar::reset_parse_state(src);
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

    SECTION("UTF-8 literals are allowed in string literals")
    {
        const std::string emoji = "😀";
        const auto expect_roundtrip = [&](const std::string& input) {
            auto result = parse(input);
            REQUIRE(result);
            auto value = std::move(result).value();
            REQUIRE(value->is<frst::String>());
            CHECK(value->get<frst::String>().value() == emoji);
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
            REQUIRE(result);
            auto value = std::move(result).value();
            REQUIRE(value->is<frst::String>());
            CHECK(value->get<frst::String>().value() == c.expected);

            INFO("single: " << c.single_input);
            auto result2 = parse(c.single_input);
            REQUIRE(result2);
            auto value2 = std::move(result2).value();
            REQUIRE(value2->is<frst::String>());
            CHECK(value2->get<frst::String>().value() == c.expected);
        }
    }

    SECTION("Hex escapes are not processed in raw strings")
    {
        auto result = parse("R\"(\\x41)\"");
        REQUIRE(result);
        CHECK(result.value()->get<frst::String>().value() == "\\x41");

        auto result2 = parse("R'(\\x41)'");
        REQUIRE(result2);
        CHECK(result2.value()->get<frst::String>().value() == "\\x41");
    }

    SECTION("Invalid hex escapes")
    {
        // Only one hex digit
        CHECK_FALSE(parse("\"\\x0\""));
        CHECK_FALSE(parse("'\\x0'"));

        // Non-hex digit after \x
        CHECK_FALSE(parse("\"\\xGG\""));
        CHECK_FALSE(parse("'\\xGG'"));

        // \x at end of string (no digits)
        CHECK_FALSE(parse("\"\\x\""));
        CHECK_FALSE(parse("'\\x'"));
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

    SECTION("Triple-quoted strings: basic multiline")
    {
        auto result = parse(R"("""
    hello
    world
    """)");
        REQUIRE(result);
        CHECK(result.value()->get<frst::String>().value() == R"(hello
world)");
    }

    SECTION("Triple-quoted strings: single-quoted variant")
    {
        auto result = parse(R"('''
    hello
    world
    ''')");
        REQUIRE(result);
        CHECK(result.value()->get<frst::String>().value() == R"(hello
world)");
    }

    SECTION("Triple-quoted strings: indentation beyond closing is preserved")
    {
        auto result = parse(R"("""
    if true:
      nested
    """)");
        REQUIRE(result);
        CHECK(result.value()->get<frst::String>().value() == R"(if true:
  nested)");
    }

    SECTION("Triple-quoted strings: empty lines preserved")
    {
        auto result = parse(R"("""
    line 1

    line 3
    """)");
        REQUIRE(result);
        CHECK(result.value()->get<frst::String>().value() == R"(line 1

line 3)");
    }

    SECTION("Triple-quoted strings: escape sequences processed")
    {
        auto result = parse(R"("""
    hello\tworld
    """)");
        REQUIRE(result);
        CHECK(result.value()->get<frst::String>().value() == "hello\tworld");
    }

    SECTION("Triple-quoted strings: hex escapes processed")
    {
        auto result = parse(R"("""
    \x48\x69
    """)");
        REQUIRE(result);
        CHECK(result.value()->get<frst::String>().value() == "Hi");
    }

    SECTION("Triple-quoted strings: no indentation on closing delimiter")
    {
        auto result = parse(R"("""
hello
world
""")");
        REQUIRE(result);
        CHECK(result.value()->get<frst::String>().value() == R"(hello
world)");
    }

    SECTION("Triple-quoted strings: single content line")
    {
        auto result = parse(R"("""
    hello
    """)");
        REQUIRE(result);
        CHECK(result.value()->get<frst::String>().value() == "hello");
    }

    SECTION("Triple-quoted strings: can contain double quotes")
    {
        auto result = parse(R"("""
    she said "hi"
    """)");
        REQUIRE(result);
        CHECK(result.value()->get<frst::String>().value()
              == R"(she said "hi")");
    }

    SECTION("Triple-quoted strings: single-quoted can contain single quotes")
    {
        auto result = parse(R"('''
    it's fine
    ''')");
        REQUIRE(result);
        CHECK(result.value()->get<frst::String>().value() == "it's fine");
    }

    SECTION("Triple-quoted strings: triple-single can contain triple-double")
    {
        auto result = parse(R"--('''
    some """quoted""" text
    ''')--");
        REQUIRE(result);
        CHECK(result.value()->get<frst::String>().value()
              == R"(some """quoted""" text)");
    }

    SECTION("Triple-quoted strings: triple-double can contain triple-single")
    {
        auto result = parse(R"("""
    some '''quoted''' text
    """)");
        REQUIRE(result);
        CHECK(result.value()->get<frst::String>().value()
              == "some '''quoted''' text");
    }

    SECTION("Triple-quoted strings: tab indentation")
    {
        auto result = parse("\"\"\"\n\thello\n\tworld\n\t\"\"\"");
        REQUIRE(result);
        CHECK(result.value()->get<frst::String>().value() == R"(hello
world)");
    }

    SECTION("Triple-quoted strings: mixed indentation levels")
    {
        auto result = parse(R"("""
    a
        b
            c
    """)");
        REQUIRE(result);
        CHECK(result.value()->get<frst::String>().value() == R"(a
    b
        c)");
    }

    SECTION("Triple-quoted strings: empty content")
    {
        auto result = parse("\"\"\"\n\"\"\"");
        REQUIRE(result);
        CHECK(result.value()->get<frst::String>().value() == "");
    }

    SECTION("Triple-quoted strings: whitespace-only content")
    {
        auto result = parse(R"("""

    """)");
        REQUIRE(result);
        CHECK(result.value()->get<frst::String>().value() == "");
    }

    SECTION("Triple-quoted strings: \\n and \\r escapes are forbidden")
    {
        auto with_n = parse(R"("""
    hello\nworld
    """)");
        CHECK_FALSE(with_n);

        auto with_r = parse(R"("""
    hello\rworld
    """)");
        CHECK_FALSE(with_r);
    }

    SECTION("Triple-quoted strings: regular strings still work")
    {
        auto regular = parse(R"("hello")");
        REQUIRE(regular);
        CHECK(regular.value()->get<frst::String>().value() == "hello");

        auto single = parse("'hello'");
        REQUIRE(single);
        CHECK(single.value()->get<frst::String>().value() == "hello");
    }
}
