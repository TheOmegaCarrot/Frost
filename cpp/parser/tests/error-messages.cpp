#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/parser.hpp>

using namespace Catch::Matchers;

namespace
{
// Asserts that the input fails to parse and returns the error text.
std::string parse_error(const std::string& input)
{
    auto result = frst::parse_program(input);
    REQUIRE_FALSE(result.has_value());
    return result.error();
}
} // namespace

TEST_CASE("Parser error messages - boolean operators")
{
    SECTION("&& suggests 'and'")
    {
        auto err = parse_error("a && b");
        CHECK_THAT(err, ContainsSubstring("use 'and' instead of '&&'"));
    }

    SECTION("|| suggests 'or'")
    {
        auto err = parse_error("a || b");
        CHECK_THAT(err, ContainsSubstring("use 'or' instead of '||'"));
    }

    SECTION("&& hint fires inside an expression context too")
    {
        // The incomplete expression `a` becomes a statement, then `&&` is
        // seen at the program level — same hint regardless.
        auto err = parse_error("def x = a\n&& b");
        CHECK_THAT(err, ContainsSubstring("use 'and' instead of '&&'"));
    }
}

TEST_CASE("Parser error messages - assignment without def")
{
    SECTION("x = 1 suggests def")
    {
        auto err = parse_error("x = 1");
        CHECK_THAT(err, ContainsSubstring("use 'def name = value'"));
    }

    SECTION("hint fires for any bare = at program level")
    {
        auto err = parse_error("foo = 42");
        CHECK_THAT(err, ContainsSubstring("use 'def name = value'"));
    }
}

TEST_CASE("Parser error messages - else if instead of elif")
{
    SECTION("else if suggests elif")
    {
        auto err = parse_error("if a: 1 else if b: 2 else: 3");
        CHECK_THAT(err, ContainsSubstring("use 'elif' instead of 'else if'"));
    }

    SECTION("valid elif still parses")
    {
        auto result = frst::parse_program("if true: 1 elif false: 2 else: 3");
        CHECK(result.has_value());
    }

    SECTION("valid else still parses")
    {
        auto result = frst::parse_program("if true: 1 else: 2");
        CHECK(result.has_value());
    }
}

TEST_CASE("Parser error messages - string literal rule name")
{
    SECTION("unclosed double-quoted string says 'string literal'")
    {
        auto err = parse_error("\"hello");
        CHECK_THAT(err, ContainsSubstring("string literal"));
        CHECK_THAT(err, !ContainsSubstring("raw_double"));
    }

    SECTION("unclosed single-quoted string says 'string literal'")
    {
        auto err = parse_error("'hello");
        CHECK_THAT(err, ContainsSubstring("string literal"));
        CHECK_THAT(err, !ContainsSubstring("raw_single"));
    }
}

TEST_CASE("Parser error messages - map entry value label")
{
    SECTION("missing value after colon says 'map entry value'")
    {
        auto err = parse_error("{a: }");
        CHECK_THAT(err, ContainsSubstring("map entry value"));
    }
}

TEST_CASE("Parser error messages - closing delimiter labels")
{
    SECTION("unclosed call says 'closing ')'")
    {
        // Empty arg list: expected_call_arguments fires immediately
        auto err = parse_error("f(");
        CHECK_THAT(err, ContainsSubstring("closing ')'"));
    }

    SECTION("unclosed index says 'closing ']'")
    {
        auto err = parse_error("a[");
        CHECK_THAT(err, ContainsSubstring("closing ']'"));
    }

    SECTION("call with args: missing ')' is still diagnosed")
    {
        auto err = parse_error("f(a");
        CHECK_THAT(err, ContainsSubstring("expected ')'"));
    }
}
