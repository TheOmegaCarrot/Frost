#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

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

frst::ast::Expression::Ptr require_expression(auto& result)
{
    auto expr = std::move(result).value();
    REQUIRE(expr);
    return expr;
}

frst::Value_Ptr call_function(const frst::Value_Ptr& value,
                              std::vector<frst::Value_Ptr> args)
{
    REQUIRE(value->is<frst::Function>());
    auto fn = value->get<frst::Function>().value();
    return fn->call(args);
}
} // namespace

TEST_CASE("Parser Lambda Expressions")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        auto src = lexy::string_input(input);
        return lexy::parse<Expression_Root>(src, lexy::noop);
    };

    SECTION("Empty body yields Null when called")
    {
        auto result = parse("fn() -> {}");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto value = expr->evaluate(table);
        auto out = call_function(value, {});
        REQUIRE(out->is<frst::Null>());
    }

    SECTION("Empty body tolerates whitespace, comments, and semicolons")
    {
        auto result = parse("fn (\n ) -> { ; # comment\n ; ; }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto value = expr->evaluate(table);
        auto out = call_function(value, {});
        REQUIRE(out->is<frst::Null>());
    }

    SECTION("Lambda parameters bind and compute correctly")
    {
        auto result = parse("fn(a, b) -> { a + b }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto value = expr->evaluate(table);
        auto out = call_function(value,
                                 {frst::Value::create(1_f),
                                  frst::Value::create(2_f)});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 3_f);
    }

    SECTION("Lambda participates in larger expressions")
    {
        auto result = parse("1 + fn() -> { 2 }()");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 3_f);
    }

    SECTION("Lambda can be called as an atom")
    {
        auto result = parse("fn(x) -> { x }(5)");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 5_f);
    }

    SECTION("Lambda can be followed by indexing and dot access after call")
    {
        frst::Symbol_Table table;
        auto arr_val = frst::Value::create(frst::Array{
            frst::Value::create(9_f),
        });
        table.define("arr", arr_val);

        frst::Map obj;
        obj.emplace(frst::Value::create(std::string{"key"}),
                    frst::Value::create(7_f));
        table.define("obj", frst::Value::create(std::move(obj)));

        auto index_result = parse("fn() -> { arr }()[0]");
        REQUIRE(index_result);
        auto index_expr = require_expression(index_result);
        auto index_out = index_expr->evaluate(table);
        REQUIRE(index_out->is<frst::Int>());
        CHECK(index_out->get<frst::Int>().value() == 9_f);

        auto dot_result = parse("fn() -> { obj }().key");
        REQUIRE(dot_result);
        auto dot_expr = require_expression(dot_result);
        auto dot_out = dot_expr->evaluate(table);
        REQUIRE(dot_out->is<frst::Int>());
        CHECK(dot_out->get<frst::Int>().value() == 7_f);
    }

    SECTION("Lambda captures outer names")
    {
        frst::Symbol_Table table;
        auto captured = frst::Value::create(10_f);
        table.define("x", captured);

        auto result = parse("fn() -> { x }");
        REQUIRE(result);
        auto expr = require_expression(result);
        auto value = expr->evaluate(table);
        auto out = call_function(value, {});
        CHECK(out == captured);
    }

    SECTION("Lambda parameters return pointer-equal values")
    {
        auto result = parse("fn(x) -> { x }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto value = expr->evaluate(table);
        auto arg = frst::Value::create(9_f);
        auto out = call_function(value, {arg});
        CHECK(out == arg);
    }

    SECTION("Lambda body supports def statements and expressions")
    {
        auto result = parse("fn() -> { def x = 1; x + 2 }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto value = expr->evaluate(table);
        auto out = call_function(value, {});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 3_f);
    }

    SECTION("Lambda body accepts multiple statements without semicolons")
    {
        auto result = parse("fn() -> { 1 2 }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto value = expr->evaluate(table);
        auto out = call_function(value, {});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 2_f);
    }

    SECTION("Lambda can contain nested lambdas")
    {
        auto result = parse("fn() -> { fn(x) -> { x + 1 } }()(5)");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 6_f);
    }

    SECTION("Lambda body can end with a definition and return Null")
    {
        auto result = parse("fn() -> { def x = 1 }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto value = expr->evaluate(table);
        auto out = call_function(value, {});
        REQUIRE(out->is<frst::Null>());
    }

    SECTION("Lambda body allows if expressions")
    {
        auto result = parse("fn() -> { if true: 1 else: 2 }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto value = expr->evaluate(table);
        auto out = call_function(value, {});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 1_f);
    }

    SECTION("Whitespace and comments around lambda tokens are allowed")
    {
        auto result = parse("fn # comment\n ( a )\n ->\n { a }\n (7)");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 7_f);
    }

    SECTION("Parameter list syntax errors are rejected")
    {
        CHECK_FALSE(parse("fn(a,) -> {}"));
        CHECK_FALSE(parse("fn(,a) -> {}"));
        CHECK_FALSE(parse("fn(a b) -> {}"));
        CHECK_FALSE(parse("fn(,) -> {}"));
    }

    SECTION("Missing arrow or body is rejected")
    {
        CHECK_FALSE(parse("fn() {}"));
        CHECK_FALSE(parse("fn() ->"));
        CHECK_FALSE(parse("fn() -> {"));
        CHECK_FALSE(parse("fn() -> }"));
    }

    SECTION("Missing braces or parameter parentheses are rejected")
    {
        CHECK_FALSE(parse("fn() -> 1"));
        CHECK_FALSE(parse("fn x -> {}"));
    }

    SECTION("Keyword parameters are rejected")
    {
        CHECK_FALSE(parse("fn(if) -> {}"));
        CHECK_FALSE(parse("fn(true) -> {}"));
        CHECK_FALSE(parse("fn(fn) -> {}"));
    }
}
