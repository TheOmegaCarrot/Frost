#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/ast.hpp>
#include <frost/parser.hpp>
#include <frost/symbol-table.hpp>
#include <frost/testing/stringmaker-specializations.hpp>
#include <frost/value.hpp>

using namespace frst::literals;

namespace
{
// Parse a single expression and evaluate it to a Value_Ptr.
// Returns nullopt if parsing fails.
std::optional<frst::Value_Ptr> parse_eval(std::string_view input)
{
    auto result = frst::parse_data(std::string{input});
    if (not result)
        return std::nullopt;
    frst::Symbol_Table table;
    frst::Evaluation_Context ctx{.symbols = table};
    return result.value()->evaluate(ctx);
}
} // namespace

TEST_CASE("Parser Numeric Literals")
{
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
            auto value = parse_eval(c.input);
            REQUIRE(value.has_value());
            REQUIRE(value.value()->is<frst::Int>());
            CHECK(value.value()->get<frst::Int>().value() == c.expected);
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
            {"0.0", 0.0},     {"1.0", 1.0},      {"3.14", 3.14},
            {"10.01", 10.01}, {"001.250", 1.25},
        };

        for (const auto& c : cases)
        {
            auto value = parse_eval(c.input);
            REQUIRE(value.has_value());
            REQUIRE(value.value()->is<frst::Float>());
            CHECK(value.value()->get<frst::Float>().value() == c.expected);
        }
    }

    SECTION("Scientific notation")
    {
        struct Case
        {
            std::string_view input;
            frst::Float expected;
        };

        const Case cases[] = {
            // decimal + signed exponent
            {"3.14e+10", 3.14e+10},
            {"1.0e-5", 1.0e-5},
            {"1.23e-10", 1.23e-10},
            {"9.99e+100", 9.99e+100},
            // decimal + unsigned exponent
            {"3.14e10", 3.14e10},
            {"1.0e2", 1.0e2},
            // integer + signed exponent
            {"1e+20", 1e+20},
            {"1e-05", 1e-05},
            {"5e+0", 5e+0},
            // integer + unsigned exponent
            {"3e2", 3e2},
            {"1e3", 1e3},
            {"5e0", 5e0},
        };

        for (const auto& c : cases)
        {
            DYNAMIC_SECTION(c.input)
            {
                auto value = parse_eval(c.input);
                REQUIRE(value.has_value());
                REQUIRE(value.value()->is<frst::Float>());
                CHECK(value.value()->get<frst::Float>().value() == c.expected);
            }
        }
    }

    SECTION("Scientific notation is Float, not Int")
    {
        auto value = parse_eval("3e2");
        REQUIRE(value.has_value());
        CHECK(value.value()->is<frst::Float>());
        CHECK_FALSE(value.value()->is<frst::Int>());
    }

    SECTION("Identifiers starting with 'e' are not floats")
    {
        auto result = frst::parse_program("e2");
        REQUIRE(result.has_value());
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);
        auto* lookup = dynamic_cast<const frst::ast::Name_Lookup*>(
            program[0].get());
        CHECK(lookup != nullptr);
    }

    SECTION("Out of range integer")
    {
        CHECK(not parse_eval("9223372036854775808"));
    }

    SECTION("Out of range float")
    {
        auto too_large = std::string(400, '9') + ".0";
        auto result = frst::parse_data(too_large);
        REQUIRE(not result);
        CHECK(result.error().find("out of range") != std::string::npos);
    }
}
