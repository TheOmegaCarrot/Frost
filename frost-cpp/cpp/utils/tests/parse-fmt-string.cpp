#include <catch2/catch_test_macros.hpp>

#include <frost/utils.hpp>

using namespace frst::utils;

TEST_CASE("utils::parse_fmt_string")
{
    SECTION("No placeholders yields a single literal")
    {
        auto result = parse_fmt_string("plain text");
        REQUIRE(result);
        REQUIRE(result->size() == 1);
        CHECK(std::holds_alternative<Fmt_Literal>(result->at(0)));
        CHECK(std::get<Fmt_Literal>(result->at(0)).text == "plain text");
    }

    SECTION("Single placeholder yields a literal and a placeholder")
    {
        auto result = parse_fmt_string("hi ${name}");
        REQUIRE(result);
        REQUIRE(result->size() == 2);
        CHECK(std::holds_alternative<Fmt_Literal>(result->at(0)));
        CHECK(std::get<Fmt_Literal>(result->at(0)).text == "hi ");
        CHECK(std::holds_alternative<Fmt_Placeholder>(result->at(1)));
        CHECK(std::get<Fmt_Placeholder>(result->at(1)).text == "name");
    }

    SECTION("Multiple and adjacent placeholders are reported in order")
    {
        auto result = parse_fmt_string("${a}${b}");
        REQUIRE(result);
        REQUIRE(result->size() == 2);
        CHECK(std::holds_alternative<Fmt_Placeholder>(result->at(0)));
        CHECK(std::get<Fmt_Placeholder>(result->at(0)).text == "a");
        CHECK(std::holds_alternative<Fmt_Placeholder>(result->at(1)));
        CHECK(std::get<Fmt_Placeholder>(result->at(1)).text == "b");

        auto result2 = parse_fmt_string("x${a}y${b}");
        REQUIRE(result2);
        REQUIRE(result2->size() == 4);
        CHECK(std::holds_alternative<Fmt_Literal>(result2->at(0)));
        CHECK(std::get<Fmt_Literal>(result2->at(0)).text == "x");
        CHECK(std::holds_alternative<Fmt_Placeholder>(result2->at(1)));
        CHECK(std::get<Fmt_Placeholder>(result2->at(1)).text == "a");
        CHECK(std::holds_alternative<Fmt_Literal>(result2->at(2)));
        CHECK(std::get<Fmt_Literal>(result2->at(2)).text == "y");
        CHECK(std::holds_alternative<Fmt_Placeholder>(result2->at(3)));
        CHECK(std::get<Fmt_Placeholder>(result2->at(3)).text == "b");
    }

    SECTION("Escaped placeholders are not reported")
    {
        auto result = parse_fmt_string("\\${name}");
        REQUIRE(result);
        REQUIRE(result->size() == 1);
        CHECK(std::holds_alternative<Fmt_Literal>(result->at(0)));
        CHECK(std::get<Fmt_Literal>(result->at(0)).text == "${name}");

        auto result2 = parse_fmt_string("x\\${a}y${b}");
        REQUIRE(result2);
        REQUIRE(result2->size() == 2);
        CHECK(std::holds_alternative<Fmt_Literal>(result2->at(0)));
        CHECK(std::get<Fmt_Literal>(result2->at(0)).text == "x${a}y");
        CHECK(std::holds_alternative<Fmt_Placeholder>(result2->at(1)));
        CHECK(std::get<Fmt_Placeholder>(result2->at(1)).text == "b");

        auto result3 = parse_fmt_string("\\\\${a}");
        REQUIRE(result3);
        REQUIRE(result3->size() == 2);
        CHECK(std::holds_alternative<Fmt_Literal>(result3->at(0)));
        CHECK(std::get<Fmt_Literal>(result3->at(0)).text == "\\");
        CHECK(std::holds_alternative<Fmt_Placeholder>(result3->at(1)));
        CHECK(std::get<Fmt_Placeholder>(result3->at(1)).text == "a");
    }

    SECTION("Dollar signs without placeholders are literal")
    {
        auto result = parse_fmt_string("$");
        REQUIRE(result);
        REQUIRE(result->size() == 1);
        CHECK(std::holds_alternative<Fmt_Literal>(result->at(0)));
        CHECK(std::get<Fmt_Literal>(result->at(0)).text == "$");

        auto result2 = parse_fmt_string("$$x");
        REQUIRE(result2);
        REQUIRE(result2->size() == 1);
        CHECK(std::holds_alternative<Fmt_Literal>(result2->at(0)));
        CHECK(std::get<Fmt_Literal>(result2->at(0)).text == "$$x");

        auto result3 = parse_fmt_string("$x");
        REQUIRE(result3);
        REQUIRE(result3->size() == 1);
        CHECK(std::holds_alternative<Fmt_Literal>(result3->at(0)));
        CHECK(std::get<Fmt_Literal>(result3->at(0)).text == "$x");
    }

    SECTION("Dollar before placeholder does not escape it")
    {
        auto result = parse_fmt_string("$${a}");
        REQUIRE(result);
        REQUIRE(result->size() == 2);
        CHECK(std::holds_alternative<Fmt_Literal>(result->at(0)));
        CHECK(std::get<Fmt_Literal>(result->at(0)).text == "$");
        CHECK(std::holds_alternative<Fmt_Placeholder>(result->at(1)));
        CHECK(std::get<Fmt_Placeholder>(result->at(1)).text == "a");
    }

    SECTION("Unterminated placeholders return an error")
    {
        auto result = parse_fmt_string("hi ${name");
        REQUIRE_FALSE(result);
        CHECK(result.error() == "Unterminated format placeholder: ${name");
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
