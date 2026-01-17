#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/symbol-table.hpp>
#include <frost/testing/stringmaker-specializations.hpp>

#include <lexy/action/parse.hpp>
#include <lexy/callback.hpp>
#include <lexy/input/string_input.hpp>

#include "../grammar.hpp"

using namespace frst::literals;

namespace
{
struct Name_Lookup_Full
{
    static constexpr auto rule =
        lexy::dsl::p<frst::grammar::node::Name_Lookup> + lexy::dsl::eof;
    static constexpr auto value = lexy::forward<frst::ast::Expression::Ptr>;
};

template <typename Result>
frst::ast::Expression::Ptr require_expression(Result& result)
{
    auto expr = std::move(result).value();
    REQUIRE(expr);
    return expr;
}
} // namespace

TEST_CASE("Name Lookup")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        auto src = lexy::string_input(input);
        return lexy::parse<Name_Lookup_Full>(src, lexy::noop);
    };

    SECTION("Valid identifiers")
    {
        struct Case
        {
            std::string_view input;
            std::string_view expected;
        };

        const Case cases[] = {
            {"foo", "foo"}, {"_bar", "_bar"}, {"a1_b2", "a1_b2"},
            {"_", "_"},     {"if1", "if1"},
        };

        for (const auto& c : cases)
        {
            auto result = parse(c.input);
            REQUIRE(result);
            auto expr = require_expression(result);

            frst::Symbol_Table table;
            auto value = frst::Value::create(42_f);
            table.define(std::string{c.expected}, value);

            auto evaluated = expr->evaluate(table);
            CHECK(evaluated == value);
        }
    }

    SECTION("Invalid identifiers fail to parse")
    {
        const std::string_view cases[] = {
            "", "1abc", "-abc", "a-b", "a b", "if", "else", "true", "null",
        };

        for (const auto& input : cases)
        {
            auto result = parse(input);
            CHECK_FALSE(result);
            CHECK(result.error_count() >= 1);
        }
    }

    SECTION("Lookup fails when identifier is not defined")
    {
        auto result = parse("missing_name");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        CHECK_THROWS_WITH(expr->evaluate(table),
                          Catch::Matchers::ContainsSubstring("not defined"));
        CHECK_THROWS_WITH(expr->evaluate(table),
                          Catch::Matchers::ContainsSubstring("missing_name"));
    }
}
