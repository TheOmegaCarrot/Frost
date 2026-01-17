#include <catch2/catch_test_macros.hpp>

#include <frost/testing/stringmaker-specializations.hpp>
#include <frost/value.hpp>

#include <lexy/action/parse.hpp>
#include <lexy/callback.hpp>
#include <lexy/input/string_input.hpp>

#include "../grammar.hpp"

using namespace frst::literals;

namespace
{
struct Integer_Root
{
    static constexpr auto whitespace = frst::grammar::ws;
    static constexpr auto rule =
        lexy::dsl::p<frst::grammar::integer_literal> + lexy::dsl::eof;
    static constexpr auto value = lexy::forward<frst::Value_Ptr>;
};

struct Float_Root
{
    static constexpr auto whitespace = frst::grammar::ws;
    static constexpr auto rule =
        lexy::dsl::p<frst::grammar::float_literal> + lexy::dsl::eof;
    static constexpr auto value = lexy::forward<frst::Value_Ptr>;
};
} // namespace

TEST_CASE("Parser Numeric Literals")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse_int = [](std::string_view input) {
        auto src = lexy::string_input(input);
        return lexy::parse<Integer_Root>(src, lexy::noop);
    };
    auto parse_float = [](std::string_view input) {
        auto src = lexy::string_input(input);
        return lexy::parse<Float_Root>(src, lexy::noop);
    };

    SECTION("Valid integers")
    {
        struct Case
        {
            std::string_view input;
            frst::Int expected;
        };

        const Case cases[] = {
            {"0", 0_f},
            {"7", 7_f},
            {"42", 42_f},
            {"001", 1_f},
            {"9223372036854775807", 9223372036854775807_f},
            {"123456", 123456_f},
        };

        for (const auto& c : cases)
        {
            auto result = parse_int(c.input);
            REQUIRE(result);
            auto value = std::move(result).value();
            REQUIRE(value->is<frst::Int>());
            CHECK(value->get<frst::Int>().value() == c.expected);
        }
    }

    SECTION("Invalid integers")
    {
        const std::string_view cases[] = {
            "",
            "1.0",
            ".1",
            "1.",
            "1e3",
            "+1",
            "-1",
            "1_0",
            "12 34",
            "0x10",
            "9223372036854775808",
        };

        for (const auto& input : cases)
        {
            auto result = parse_int(input);
            CHECK_FALSE(result);
        }
    }

    SECTION("Valid floats")
    {
        struct Case
        {
            std::string_view input;
            frst::Float expected;
        };

        const Case cases[] = {
            {"0.0", 0.0},
            {"1.0", 1.0},
            {"3.14", 3.14},
            {"10.01", 10.01},
            {"001.250", 1.25},
        };

        for (const auto& c : cases)
        {
            auto result = parse_float(c.input);
            REQUIRE(result);
            auto value = std::move(result).value();
            REQUIRE(value->is<frst::Float>());
            CHECK(value->get<frst::Float>().value() == c.expected);
        }
    }

    SECTION("Invalid floats")
    {
        const std::string_view cases[] = {
            "",
            "1",
            "1.",
            ".1",
            "1.2.3",
            "1e3",
            "1.0e2",
            "+1.0",
            "-1.0",
            "1_0.2",
            "12 34",
        };

        for (const auto& input : cases)
        {
            auto result = parse_float(input);
            CHECK_FALSE(result);
        }
    }

    SECTION("Out of range floats")
    {
        auto too_large = std::string(400, '9') + ".0";
        auto result = parse_float(too_large);
        CHECK_FALSE(result);
    }
}
