#include <catch2/catch_test_macros.hpp>

#include <frost/utils.hpp>

using namespace frst::utils;

TEST_CASE("utils::parse_fmt_string")
{
    SECTION("No placeholders yields an empty list")
    {
        auto result = parse_fmt_string("plain text");
        REQUIRE(result);
        CHECK(result->empty());
    }

    SECTION("Single placeholder yields one section")
    {
        auto result = parse_fmt_string("hi ${name}");
        REQUIRE(result);
        REQUIRE(result->size() == 1);
        CHECK(result->at(0).start == 3);
        CHECK(result->at(0).len == 7);
        CHECK(result->at(0).content == "name");
    }

    SECTION("Multiple and adjacent placeholders are reported in order")
    {
        auto result = parse_fmt_string("${a}${b}");
        REQUIRE(result);
        REQUIRE(result->size() == 2);
        CHECK(result->at(0).start == 0);
        CHECK(result->at(0).len == 4);
        CHECK(result->at(0).content == "a");
        CHECK(result->at(1).start == 4);
        CHECK(result->at(1).len == 4);
        CHECK(result->at(1).content == "b");

        auto result2 = parse_fmt_string("x${a}y${b}");
        REQUIRE(result2);
        REQUIRE(result2->size() == 2);
        CHECK(result2->at(0).start == 1);
        CHECK(result2->at(0).len == 4);
        CHECK(result2->at(0).content == "a");
        CHECK(result2->at(1).start == 6);
        CHECK(result2->at(1).len == 4);
        CHECK(result2->at(1).content == "b");
    }

    SECTION("Escaped placeholders are not reported")
    {
        auto result = parse_fmt_string("$${name}");
        REQUIRE(result);
        CHECK(result->empty());

        auto result2 = parse_fmt_string("x$${a}y${b}");
        REQUIRE(result2);
        REQUIRE(result2->size() == 1);
        CHECK(result2->at(0).start == 7);
        CHECK(result2->at(0).len == 4);
        CHECK(result2->at(0).content == "b");

        auto result3 = parse_fmt_string("$$${a}");
        REQUIRE(result3);
        REQUIRE(result3->size() == 1);
        CHECK(result3->at(0).start == 2);
        CHECK(result3->at(0).len == 4);
        CHECK(result3->at(0).content == "a");
    }

    SECTION("Dollar signs without placeholders are literal")
    {
        auto result = parse_fmt_string("$");
        REQUIRE(result);
        CHECK(result->empty());

        auto result2 = parse_fmt_string("$$x");
        REQUIRE(result2);
        CHECK(result2->empty());

        auto result3 = parse_fmt_string("$x");
        REQUIRE(result3);
        CHECK(result3->empty());
    }

    SECTION("Unterminated placeholders return an error")
    {
        auto result = parse_fmt_string("hi ${name");
        REQUIRE_FALSE(result);
        CHECK(result.error() ==
              "Unterminated format placeholder: missing '}'");
    }

    SECTION("Empty placeholders return an error")
    {
        auto result = parse_fmt_string("${}");
        REQUIRE_FALSE(result);
        CHECK(result.error() == "Invalid format placeholder: ${}");
    }

    SECTION("Invalid placeholder identifiers return an error")
    {
        auto result = parse_fmt_string("${1abc}");
        REQUIRE_FALSE(result);
        CHECK(result.error() == "Invalid format placeholder: ${1abc}");

        auto result2 = parse_fmt_string("${a-b}");
        REQUIRE_FALSE(result2);
        CHECK(result2.error() == "Invalid format placeholder: ${a-b}");
    }

    SECTION("Nested placeholders return an error")
    {
        auto result = parse_fmt_string("${${a}}");
        REQUIRE_FALSE(result);
        CHECK(result.error() == "Invalid format placeholder: ${${a}");
    }
}
