#include <catch2/catch_test_macros.hpp>

#include <frost/symbol-table.hpp>
#include <frost/testing/stringmaker-specializations.hpp>
#include <frost/value.hpp>

#include <lexy/action/parse.hpp>
#include <lexy/callback.hpp>
#include <lexy/input/string_input.hpp>

#include "../grammar.hpp"

using namespace frst::literals;

namespace
{
struct Expression_Root
{
    static constexpr auto whitespace = frst::grammar::ws;
    static constexpr auto rule =
        lexy::dsl::p<frst::grammar::expression> + lexy::dsl::eof;
    static constexpr auto value = lexy::forward<frst::ast::Expression::Ptr>;
};

struct Program_Root
{
    static constexpr auto rule = lexy::dsl::p<frst::grammar::program>;
    static constexpr auto value =
        lexy::forward<std::vector<frst::ast::Statement::Ptr>>;
};

frst::ast::Expression::Ptr require_expression(auto& result)
{
    auto expr = std::move(result).value();
    REQUIRE(expr);
    return expr;
}

frst::Value_Ptr evaluate_expression(const frst::ast::Statement::Ptr& statement,
                                    frst::Symbol_Table& table)
{
    auto* expr = dynamic_cast<const frst::ast::Expression*>(statement.get());
    REQUIRE(expr);
    return expr->evaluate(table);
}

struct IdentityCallable final : frst::Callable
{
    frst::Value_Ptr call(std::span<const frst::Value_Ptr> args) const override
    {
        if (args.empty())
        {
            return frst::Value::null();
        }
        return args.front();
    }

    std::string debug_dump() const override
    {
        return "<identity>";
    }
};
} // namespace

TEST_CASE("Parser Array Literals")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        auto src = lexy::string_input(input);
        return lexy::parse<Expression_Root>(src, lexy::noop);
    };

    SECTION("Empty array literal is valid")
    {
        auto result = parse("[]");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Array>());
        CHECK(out->raw_get<frst::Array>().empty());
    }

    SECTION("Array elements preserve order")
    {
        auto result = parse("[1, 2, 3]");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 3);
        CHECK(arr[0]->get<frst::Int>().value() == 1_f);
        CHECK(arr[1]->get<frst::Int>().value() == 2_f);
        CHECK(arr[2]->get<frst::Int>().value() == 3_f);
    }

    SECTION("Trailing commas are accepted")
    {
        auto result = parse("[1, 2,]\n");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 2);
        CHECK(arr[0]->get<frst::Int>().value() == 1_f);
        CHECK(arr[1]->get<frst::Int>().value() == 2_f);
    }

    SECTION("Commas are required between elements")
    {
        CHECK_FALSE(parse("[1 2]"));
    }

    SECTION("Whitespace and comments are allowed between elements")
    {
        auto result = parse("[ 1 , # comment\n 2 ,\n 3 ]");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 3);
        CHECK(arr[0]->get<frst::Int>().value() == 1_f);
        CHECK(arr[1]->get<frst::Int>().value() == 2_f);
        CHECK(arr[2]->get<frst::Int>().value() == 3_f);
    }

    SECTION("Nested arrays parse and evaluate")
    {
        auto result = parse("[[1], [2, 3]]");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 2);
        REQUIRE(arr[0]->is<frst::Array>());
        REQUIRE(arr[1]->is<frst::Array>());
        CHECK(arr[0]->raw_get<frst::Array>().size() == 1);
        CHECK(arr[1]->raw_get<frst::Array>().size() == 2);
    }

    SECTION("Array elements can be complex expressions")
    {
        auto result = parse("[1+2, if true: 3 else: 4, not false]");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 3);
        REQUIRE(arr[0]->is<frst::Int>());
        REQUIRE(arr[1]->is<frst::Int>());
        REQUIRE(arr[2]->is<frst::Bool>());
        CHECK(arr[0]->get<frst::Int>().value() == 3_f);
        CHECK(arr[1]->get<frst::Int>().value() == 3_f);
        CHECK(arr[2]->get<frst::Bool>().value() == true);
    }

    SECTION("Array literals can be passed as function arguments")
    {
        frst::Symbol_Table table;
        auto callable = std::make_shared<IdentityCallable>();
        table.define("id", frst::Value::create(frst::Function{callable}));

        auto result = parse("id([1, 2])");
        REQUIRE(result);
        auto expr = require_expression(result);

        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 2);
        CHECK(arr[0]->get<frst::Int>().value() == 1_f);
        CHECK(arr[1]->get<frst::Int>().value() == 2_f);
    }

    SECTION("Array literals can be used as index expressions")
    {
        auto result = parse("arr[[0]]");
        REQUIRE(result);
    }

    SECTION("Array literals can participate in postfix and prefix")
    {
        auto result = parse("-[1, 2][0]");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == -1_f);
    }

    SECTION("Array literals can be used in conditions and unary ops")
    {
        auto result = parse("if [1]: 2 else: 3");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 2_f);

        auto result2 = parse("not [1]");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto out2 = expr2->evaluate(table);
        REQUIRE(out2->is<frst::Bool>());
        CHECK(out2->get<frst::Bool>().value() == false);
    }

    SECTION("Arrays can start statements in programs")
    {
        auto src = lexy::string_input(std::string_view{"[]\n42"});
        auto program_result =
            lexy::parse<Program_Root>(src, lexy::noop);
        REQUIRE(program_result);
        auto program = std::move(program_result).value();
        REQUIRE(program.size() == 2);

        frst::Symbol_Table table;
        auto first = evaluate_expression(program[0], table);
        auto second = evaluate_expression(program[1], table);
        REQUIRE(first->is<frst::Array>());
        REQUIRE(second->is<frst::Int>());
        CHECK(second->get<frst::Int>().value() == 42_f);
    }

    SECTION("Trailing comma with comments is accepted")
    {
        auto result = parse("[1, 2, # c\n]");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 2);
    }

    SECTION("Malformed arrays are rejected")
    {
        CHECK_FALSE(parse("[1,2"));
        CHECK_FALSE(parse("[1,,2]"));
        CHECK_FALSE(parse("[,]"));
        CHECK_FALSE(parse("[,1]"));
        CHECK_FALSE(parse("[1, ,2]"));
        CHECK_FALSE(parse("[1,,]"));
        CHECK_FALSE(parse("[1,2,,]"));
        CHECK_FALSE(parse("[1;2]"));
        CHECK_FALSE(parse("[1; 2, 3]"));
        CHECK_FALSE(parse("[1, 2; 3]"));
        CHECK_FALSE(parse("[1;]"));
        CHECK_FALSE(parse("[;1]"));
    }
}
