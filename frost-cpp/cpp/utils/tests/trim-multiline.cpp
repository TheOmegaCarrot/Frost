#include <catch2/catch_test_macros.hpp>

#include <frost/utils.hpp>

using namespace std::literals;
using frst::utils::trim_multiline_indentation;

TEST_CASE("utils::trim_multiline_indentation")
{
    SECTION("Single-line content passes through unchanged")
    {
        CHECK(trim_multiline_indentation("hello").value() == "hello");
        CHECK(trim_multiline_indentation("").value() == "");
        CHECK(trim_multiline_indentation("  spaced  ").value() == "  spaced  ");
    }

    SECTION("Basic multiline trimming")
    {
        // Simulates:
        //   """
        //   hello
        //   world
        //   """
        // Content between delimiters: "\nhello\nworld\n"
        // Closing delimiter at column 0.
        auto result = trim_multiline_indentation("\nhello\nworld\n");
        REQUIRE(result.has_value());
        CHECK(result.value() == "hello\nworld");
    }

    SECTION("Indented closing delimiter strips prefix")
    {
        // Simulates:
        //     """
        //     hello
        //     world
        //     """
        // Content: "\n    hello\n    world\n    "
        auto result =
            trim_multiline_indentation("\n    hello\n    world\n    ");
        REQUIRE(result.has_value());
        CHECK(result.value() == "hello\nworld");
    }

    SECTION("Content indented beyond closing delimiter is preserved")
    {
        // Simulates:
        //   """
        //   if true:
        //     nested
        //   """
        auto result =
            trim_multiline_indentation("\n  if true:\n    nested\n  ");
        REQUIRE(result.has_value());
        CHECK(result.value() == "if true:\n  nested");
    }

    SECTION("Empty lines are preserved")
    {
        auto result =
            trim_multiline_indentation("\n    hello\n\n    world\n    ");
        REQUIRE(result.has_value());
        CHECK(result.value() == "hello\n\nworld");
    }

    SECTION("Single line between delimiters")
    {
        auto result = trim_multiline_indentation("\n    hello\n    ");
        REQUIRE(result.has_value());
        CHECK(result.value() == "hello");
    }

    SECTION("Tab indentation")
    {
        auto result = trim_multiline_indentation("\n\thello\n\tworld\n\t");
        REQUIRE(result.has_value());
        CHECK(result.value() == "hello\nworld");
    }

    SECTION("Error: content to the left of closing delimiter")
    {
        auto result = trim_multiline_indentation("\noops\n  ");
        REQUIRE_FALSE(result.has_value());
        CHECK(result.error().find("indented less") != std::string::npos);
    }

    SECTION("Error: closing delimiter not on its own line")
    {
        // Content on the last line (no trailing newline before delimiter)
        auto result = trim_multiline_indentation("\nhello\nworld");
        REQUIRE_FALSE(result.has_value());
        CHECK(result.error().find("closing delimiter") != std::string::npos);
    }
}
