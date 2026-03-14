#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/parser.hpp>

using namespace frst;
using Catch::Matchers::ContainsSubstring;

TEST_CASE("parse_data")
{
    SECTION("Parses integer literal")
    {
        auto result = parse_data("42");
        REQUIRE(result.has_value());
        CHECK(result.value()->node_label() == "Literal(42) [1:1-1:2]");
    }

    SECTION("Parses float literal")
    {
        auto result = parse_data("3.14");
        REQUIRE(result.has_value());
        CHECK(result.value()->node_label().starts_with("Literal(3.14"));
    }

    SECTION("Parses string literal")
    {
        auto result = parse_data(R"("hello")");
        REQUIRE(result.has_value());
        CHECK(result.value()->node_label() == R"(Literal("hello") [1:1-1:7])");
    }

    SECTION("Parses bool literals")
    {
        REQUIRE(parse_data("true").has_value());
        REQUIRE(parse_data("false").has_value());
    }

    SECTION("Parses null")
    {
        REQUIRE(parse_data("null").has_value());
    }

    SECTION("Parses array constructor")
    {
        auto result = parse_data("[1, 2, 3]");
        REQUIRE(result.has_value());
        CHECK(result.value()->node_label() == "Array_Constructor [1:1-1:9]");
    }

    SECTION("Parses map constructor")
    {
        auto result = parse_data("{foo: 1}");
        REQUIRE(result.has_value());
        CHECK(result.value()->node_label() == "Map_Constructor [1:1-1:8]");
    }

    SECTION("Parses arithmetic expression")
    {
        REQUIRE(parse_data("1 + 2").has_value());
        REQUIRE(parse_data("-1").has_value());
    }

    SECTION("Parses name lookup (data_safe check is the caller's concern)")
    {
        // parse_data only validates syntax; safety validation is in read_value
        REQUIRE(parse_data("foo").has_value());
    }

    SECTION("Rejects statements")
    {
        // def is a statement, not an expression
        auto result = parse_data("def x = 1");
        REQUIRE_FALSE(result.has_value());
    }

    SECTION("Rejects syntax errors")
    {
        auto result = parse_data("[1, 2");
        REQUIRE_FALSE(result.has_value());
    }

    SECTION("EOF is enforced: rejects trailing tokens")
    {
        auto result = parse_data("1 2");
        REQUIRE_FALSE(result.has_value());
    }

    SECTION("Rejects empty input")
    {
        auto result = parse_data("");
        REQUIRE_FALSE(result.has_value());
    }
}
