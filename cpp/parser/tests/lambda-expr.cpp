#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/ast.hpp>
#include <frost/closure.hpp>
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
    std::string name() const override
    {
        return debug_dump();
    }
};
} // namespace

TEST_CASE("Parser Lambda Expressions")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        auto src = lexy::string_input<lexy::utf8_encoding>(input);
        frst::grammar::reset_parse_state(src);
        return lexy::parse<Expression_Root>(src, lexy::noop);
    };

    SECTION("Empty braces parse as an empty-map-returning lambda")
    {
        auto returns_empty_map = [&](std::string_view src,
                                     std::vector<frst::Value_Ptr> args = {}) {
            auto result = parse(src);
            REQUIRE(result);
            auto expr = require_expression(result);
            frst::Symbol_Table table;
            frst::Evaluation_Context eval_ctx{.symbols = table};
            auto value = expr->evaluate(eval_ctx);
            auto out = call_function(value, std::move(args));
            REQUIRE(out->is<frst::Map>());
            CHECK(out->raw_get<frst::Map>().empty());
        };
        returns_empty_map("fn() -> {}");
        returns_empty_map("fn -> {}");
        returns_empty_map("fn(x) -> {}", {frst::Value::null()});
    }

    SECTION("Empty parameter list can be elided")
    {
        auto result = parse("fn -> { 2 }()");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto out = expr->evaluate(eval_ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 2_f);
    }

    SECTION("Elided empty parameter list tolerates whitespace")
    {
        auto result = parse("fn  -> { 3 } ( )");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto out = expr->evaluate(eval_ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 3_f);
    }

    SECTION("Semicolon-only lambda body is rejected")
    {
        CHECK_THROWS_WITH(parse("fn (\n ) -> { ; # comment\n ; ; }"),
                          Catch::Matchers::ContainsSubstring("empty body"));
    }

    SECTION("Lambda parameters bind and compute correctly")
    {
        auto result = parse("fn(a, b) -> { a + b }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(
            value, {frst::Value::create(1_f), frst::Value::create(2_f)});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 3_f);
    }

    SECTION("Elided parameter list with one parameter")
    {
        auto result = parse("fn x -> x + 2");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(value, {frst::Value::create(2_f)});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 4_f);
    }

    SECTION("Elided parameter list with multiple parameters")
    {
        auto result = parse("fn a, b -> a + b");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(
            value, {frst::Value::create(1_f), frst::Value::create(3_f)});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 4_f);
    }

    SECTION("Elided parameter list with variadic parameter")
    {
        auto result = parse("fn a, ...rest -> rest");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(value, {frst::Value::create(1_f),
                                         frst::Value::create(2_f),
                                         frst::Value::create(3_f)});
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 2);
        CHECK(arr[0]->get<frst::Int>().value() == 2_f);
        CHECK(arr[1]->get<frst::Int>().value() == 3_f);
    }

    SECTION("Elided parameter list with only variadic parameter")
    {
        auto result = parse("fn ...rest -> rest");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(value, {frst::Value::create(9_f)});
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 1);
        CHECK(arr[0]->get<frst::Int>().value() == 9_f);
    }

    SECTION("Elided parameter list tolerates whitespace")
    {
        auto result = parse("fn  a ,   b -> a + b");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(
            value, {frst::Value::create(2_f), frst::Value::create(5_f)});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 7_f);
    }

    SECTION("Variadic-only parameter collects all args")
    {
        auto result = parse("fn(...rest) -> { rest }(1, 2)");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto out = expr->evaluate(eval_ctx);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 2);
        CHECK(arr[0]->get<frst::Int>().value() == 1_f);
        CHECK(arr[1]->get<frst::Int>().value() == 2_f);
    }

    SECTION("Variadic-only parameter can be empty")
    {
        auto result = parse("fn(...rest) -> { rest }()");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto out = expr->evaluate(eval_ctx);
        REQUIRE(out->is<frst::Array>());
        CHECK(out->raw_get<frst::Array>().empty());
    }

    SECTION("Variadic parameter splits fixed args from rest")
    {
        auto result = parse("fn(a, ...rest) -> { a }(1, 2, 3)");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto out = expr->evaluate(eval_ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 1_f);
    }

    SECTION("Variadic parameter binds remaining args into an array")
    {
        auto result = parse("fn(a, ...rest) -> { rest }(1, 2, 3)");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto out = expr->evaluate(eval_ctx);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 2);
        CHECK(arr[0]->get<frst::Int>().value() == 2_f);
        CHECK(arr[1]->get<frst::Int>().value() == 3_f);
    }

    SECTION("Variadic parameter can be empty")
    {
        auto result = parse("fn(a, ...rest) -> { rest }(1)");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto out = expr->evaluate(eval_ctx);
        REQUIRE(out->is<frst::Array>());
        CHECK(out->raw_get<frst::Array>().empty());
    }

    SECTION("Variadic parameter name tolerates whitespace")
    {
        auto result = parse("fn(... rest) -> { rest }(1)");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto out = expr->evaluate(eval_ctx);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 1);
        CHECK(arr[0]->get<frst::Int>().value() == 1_f);
    }

    SECTION("Variadic-only lambda works in program input")
    {
        auto src = lexy::string_input<lexy::utf8_encoding>(
            std::string_view{"def f = fn(...rest) -> { rest }\n f()"});
        frst::grammar::reset_parse_state(src);
        auto program_result =
            lexy::parse<frst::grammar::program>(src, lexy::noop);
        REQUIRE(program_result);
        auto program = std::move(program_result).value();
        REQUIRE(program.size() == 2);

        frst::Symbol_Table table;
        frst::Execution_Context ctx{.symbols = table};
        frst::Evaluation_Context eval_ctx{.symbols = table};
        program[0]->execute(ctx);
        auto* expr =
            dynamic_cast<const frst::ast::Expression*>(program[1].get());
        REQUIRE(expr);
        auto out = expr->evaluate(eval_ctx);
        REQUIRE(out->is<frst::Array>());
        CHECK(out->raw_get<frst::Array>().empty());
    }

    SECTION("Lambda participates in larger expressions")
    {
        auto result = parse("1 + fn() -> { 2 }()");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto out = expr->evaluate(eval_ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 3_f);
    }

    SECTION("Lambda as a top-level statement parses in program input")
    {
        auto src = lexy::string_input<lexy::utf8_encoding>(
            std::string_view{"fn -> { null }"});
        frst::grammar::reset_parse_state(src);
        auto program_result =
            lexy::parse<frst::grammar::program>(src, lexy::noop);
        REQUIRE(program_result);
        auto program = std::move(program_result).value();
        REQUIRE(program.size() == 1);
        auto* expr =
            dynamic_cast<const frst::ast::Expression*>(program.front().get());
        REQUIRE(expr);
    }

    SECTION(
        "Top-level empty-braces lambda parses as empty-map-returning lambda")
    {
        auto src = lexy::string_input<lexy::utf8_encoding>(
            std::string_view{"fn -> {}"});
        frst::grammar::reset_parse_state(src);
        auto result = lexy::parse<frst::grammar::program>(src, lexy::noop);
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);
        auto* expr =
            dynamic_cast<const frst::ast::Expression*>(program.front().get());
        REQUIRE(expr);
    }

    SECTION("Multiple lambdas as top-level statements parse correctly")
    {
        auto src = lexy::string_input<lexy::utf8_encoding>(
            std::string_view{"fn -> { null }\nfn() -> { null }"});
        frst::grammar::reset_parse_state(src);
        auto program_result =
            lexy::parse<frst::grammar::program>(src, lexy::noop);
        REQUIRE(program_result);
        auto program = std::move(program_result).value();
        REQUIRE(program.size() == 2);
    }

    SECTION("Multiple lambdas with semicolons parse correctly")
    {
        auto src = lexy::string_input<lexy::utf8_encoding>(
            std::string_view{"fn -> { null }; fn() -> { null }"});
        frst::grammar::reset_parse_state(src);
        auto program_result =
            lexy::parse<frst::grammar::program>(src, lexy::noop);
        REQUIRE(program_result);
        auto program = std::move(program_result).value();
        REQUIRE(program.size() == 2);
    }

    SECTION("Lambda can appear on the RHS of def")
    {
        auto src = lexy::string_input<lexy::utf8_encoding>(
            std::string_view{"def f = fn(x) -> { x }\n f(3)"});
        frst::grammar::reset_parse_state(src);
        auto program_result =
            lexy::parse<frst::grammar::program>(src, lexy::noop);
        REQUIRE(program_result);
        auto program = std::move(program_result).value();
        REQUIRE(program.size() == 2);

        frst::Symbol_Table table;
        frst::Execution_Context ctx{.symbols = table};
        frst::Evaluation_Context eval_ctx{.symbols = table};
        program[0]->execute(ctx);
        auto* expr =
            dynamic_cast<const frst::ast::Expression*>(program[1].get());
        REQUIRE(expr);
        auto out = expr->evaluate(eval_ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 3_f);
    }

    SECTION("Lambda in binary expression without call parses")
    {
        auto result = parse("1 + fn() -> { 2 }");
        REQUIRE(result);
    }

    SECTION("Lambda can be passed as a call argument")
    {
        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto callable = std::make_shared<IdentityCallable>();
        table.define("id", frst::Value::create(frst::Function{callable}));

        auto result = parse("id(fn(x) -> { x })");
        REQUIRE(result);
        auto expr = require_expression(result);

        auto out = expr->evaluate(eval_ctx);
        REQUIRE(out->is<frst::Function>());
        auto fn = out->get<frst::Function>().value();
        auto fn_out = fn->call({frst::Value::create(5_f)});
        REQUIRE(fn_out->is<frst::Int>());
        CHECK(fn_out->get<frst::Int>().value() == 5_f);
    }

    SECTION("Lambda can appear inside indexing and call arguments")
    {
        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto arr_val = frst::Value::create(frst::Array{
            frst::Value::create(11_f),
        });
        table.define("arr", arr_val);

        auto callable = std::make_shared<IdentityCallable>();
        table.define("id", frst::Value::create(frst::Function{callable}));

        auto result = parse("arr[fn() -> { 0 }()]");
        REQUIRE(result);
        auto expr = require_expression(result);
        auto out = expr->evaluate(eval_ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 11_f);

        auto result2 = parse("id(fn() -> { 3 }())");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto out2 = expr2->evaluate(eval_ctx);
        REQUIRE(out2->is<frst::Int>());
        CHECK(out2->get<frst::Int>().value() == 3_f);
    }

    SECTION("Lambda participates in comparisons and logical ops")
    {
        auto result = parse("fn() -> { 1 }() < 2");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto out = expr->evaluate(eval_ctx);
        REQUIRE(out->is<frst::Bool>());
        CHECK(out->get<frst::Bool>().value() == true);

        auto result2 = parse("fn() -> { true }() and false");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto out2 = expr2->evaluate(eval_ctx);
        REQUIRE(out2->is<frst::Bool>());
        CHECK(out2->get<frst::Bool>().value() == false);
    }

    SECTION("Prefix operators apply to lambda calls")
    {
        auto result = parse("-fn() -> { 2 }()");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto out = expr->evaluate(eval_ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == -2_f);

        auto result2 = parse("not fn() -> { true }()");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto out2 = expr2->evaluate(eval_ctx);
        REQUIRE(out2->is<frst::Bool>());
        CHECK(out2->get<frst::Bool>().value() == false);
    }

    SECTION("Lambda can be called as an atom")
    {
        auto result = parse("fn(x) -> { x }(5)");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto out = expr->evaluate(eval_ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 5_f);
    }

    SECTION("Whitespace between tokens around arrow and body is allowed")
    {
        auto result = parse("fn()  -> { 1 }()");
        REQUIRE(result);
        auto expr = require_expression(result);
        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto out = expr->evaluate(eval_ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 1_f);
    }

    SECTION("Block body can start on a new line")
    {
        auto result = parse("fn (x) ->\n{\n    def y = x + 1\n    y\n}");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(value, {frst::Value::create(2_f)});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 3_f);
    }

    SECTION("Expression body can start on a new line")
    {
        auto result = parse("fn (x) ->\n # comment\n x");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(value, {frst::Value::create(7_f)});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 7_f);
    }

    SECTION("Identifier-key map literal as lambda body")
    {
        auto check_map_a1 = [&](std::string_view src) {
            auto result = parse(src);
            REQUIRE(result);
            auto expr = require_expression(result);
            frst::Symbol_Table table;
            frst::Evaluation_Context eval_ctx{.symbols = table};
            auto value = expr->evaluate(eval_ctx);
            auto out = call_function(value, {});
            REQUIRE(out->is<frst::Map>());
            const auto& map = out->raw_get<frst::Map>();
            REQUIRE(map.size() == 1);
            auto key = frst::Value::create(std::string{"a"});
            auto it = map.find(key);
            REQUIRE(it != map.end());
            REQUIRE(it->second->is<frst::Int>());
            CHECK(it->second->get<frst::Int>().value() == 1_f);
        };
        // Bare map literal — now accepted without parentheses
        check_map_a1("fn -> {a: 1}");
        // Parenthesized form still works
        check_map_a1("fn -> ({a: 1})");
    }

    SECTION("Postfix can follow a lambda with intervening whitespace")
    {
        auto result = parse("fn() -> { 1 } ( )");
        REQUIRE(result);
        auto expr = require_expression(result);
        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto out = expr->evaluate(eval_ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 1_f);
    }

    SECTION("Postfix with newline after lambda body is rejected")
    {
        frst::Symbol_Table table;
        auto arr_val = frst::Value::create(frst::Array{
            frst::Value::create(5_f),
        });
        table.define("arr", arr_val);

        auto result = parse("fn() -> { arr }\n[0]");
        REQUIRE_FALSE(result);
    }

    SECTION("Lambda can be followed by indexing and dot access after call")
    {
        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
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
        auto index_out = index_expr->evaluate(eval_ctx);
        REQUIRE(index_out->is<frst::Int>());
        CHECK(index_out->get<frst::Int>().value() == 9_f);

        auto dot_result = parse("fn() -> { obj }().key");
        REQUIRE(dot_result);
        auto dot_expr = require_expression(dot_result);
        auto dot_out = dot_expr->evaluate(eval_ctx);
        REQUIRE(dot_out->is<frst::Int>());
        CHECK(dot_out->get<frst::Int>().value() == 7_f);
    }

    SECTION("Lambda can be followed by postfix operators without call")
    {
        frst::Symbol_Table table;
        auto arr_val = frst::Value::create(frst::Array{
            frst::Value::create(4_f),
        });
        table.define("arr", arr_val);

        frst::Map obj;
        obj.emplace(frst::Value::create(std::string{"k"}),
                    frst::Value::create(6_f));
        table.define("obj", frst::Value::create(std::move(obj)));

        auto parse_index = parse("(fn() -> { arr })[0]");
        REQUIRE(parse_index);

        auto parse_dot = parse("(fn() -> { obj }).k");
        REQUIRE(parse_dot);
    }

    SECTION("Lambda captures outer names")
    {
        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto captured = frst::Value::create(10_f);
        table.define("x", captured);

        auto result = parse("fn() -> { x }");
        REQUIRE(result);
        auto expr = require_expression(result);
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(value, {});
        CHECK(out == captured);
    }

    SECTION("Lambda parameters return pointer-equal values")
    {
        auto result = parse("fn(x) -> { x }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
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
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(value, {});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 3_f);
    }

    SECTION("Export def is rejected inside lambda bodies")
    {
        auto result = parse("fn() -> { export def x = 1 }");
        CHECK_FALSE(result);
        CHECK(result.error_count() >= 1);
    }

    SECTION("Lambda body supports defs and map literals with nested lambdas")
    {
        auto src = lexy::string_input<lexy::utf8_encoding>(
            std::string_view{"def f = fn a -> {\n"
                             "    def b = 2 + a\n"
                             "    { c: b, d: fn p -> p * a }\n"
                             "}\n"});
        frst::grammar::reset_parse_state(src);
        auto program_result =
            lexy::parse<frst::grammar::program>(src, lexy::noop);
        REQUIRE(program_result);
        auto program = std::move(program_result).value();
        REQUIRE(program.size() == 1);
    }

    SECTION("Single-expression body without braces is accepted")
    {
        auto result = parse("fn(x) -> x + 2");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(value, {frst::Value::create(3_f)});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 5_f);
    }

    SECTION("Elided body supports unary operators")
    {
        auto result = parse("fn() -> -1");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(value, {});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == -1_f);

        auto result2 = parse("fn() -> not false");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto value2 = expr2->evaluate(eval_ctx);
        auto out2 = call_function(value2, {});
        REQUIRE(out2->is<frst::Bool>());
        CHECK(out2->get<frst::Bool>().value() == true);
    }

    SECTION("Elided body supports if expressions")
    {
        auto result = parse("fn() -> if true: 1 else: 2");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(value, {});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 1_f);
    }

    SECTION("Elided body can return a lambda")
    {
        auto result = parse("fn() -> fn(x) -> x + 1");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto outer = expr->evaluate(eval_ctx);
        REQUIRE(outer->is<frst::Function>());
        auto outer_out = call_function(outer, {});
        REQUIRE(outer_out->is<frst::Function>());
        auto inner_out = call_function(outer_out, {frst::Value::create(5_f)});
        REQUIRE(inner_out->is<frst::Int>());
        CHECK(inner_out->get<frst::Int>().value() == 6_f);
    }

    SECTION("Elided body supports indexing and dot access")
    {
        auto result = parse("fn() -> [1, 2, 3][1]");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(value, {});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 2_f);

        auto result2 = parse("fn() -> ({foo: 3}).foo");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto value2 = expr2->evaluate(eval_ctx);
        auto out2 = call_function(value2, {});
        REQUIRE(out2->is<frst::Int>());
        CHECK(out2->get<frst::Int>().value() == 3_f);
    }

    SECTION("Elided body supports calling a parameter")
    {
        auto result = parse("fn(f) -> f(3)");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto callable = std::make_shared<IdentityCallable>();
        auto fn_value = frst::Value::create(frst::Function{callable});
        auto out = call_function(value, {fn_value});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 3_f);
    }

    SECTION("Elided body can be immediately called with parentheses")
    {
        auto result = parse("(fn(x) -> x + 2)(3)");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto out = expr->evaluate(eval_ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 5_f);
    }

    SECTION("Elided body can appear in larger expressions with parentheses")
    {
        auto result = parse("1 + (fn(x) -> x + 2)(3)");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto out = expr->evaluate(eval_ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 6_f);
    }

    SECTION("Elided body works in program input")
    {
        auto src = lexy::string_input<lexy::utf8_encoding>(
            std::string_view{"def f = fn(x) -> x + 2\n f(4)"});
        frst::grammar::reset_parse_state(src);
        auto program_result =
            lexy::parse<frst::grammar::program>(src, lexy::noop);
        REQUIRE(program_result);
        auto program = std::move(program_result).value();
        REQUIRE(program.size() == 2);

        frst::Symbol_Table table;
        frst::Execution_Context ctx{.symbols = table};
        frst::Evaluation_Context eval_ctx{.symbols = table};
        program[0]->execute(ctx);
        auto* expr =
            dynamic_cast<const frst::ast::Expression*>(program[1].get());
        REQUIRE(expr);
        auto out = expr->evaluate(eval_ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 6_f);
    }

    SECTION("Elided body does not allow def statements")
    {
        CHECK_FALSE(parse("fn() -> def x = 1"));
    }

    SECTION("Elided body does not allow multiple expressions")
    {
        CHECK_FALSE(parse("fn() -> 1 2"));
    }

    SECTION("Lambda body allows leading/trailing semicolons")
    {
        auto result = parse("fn() -> { ;; 1 ;; 2 ;; }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(value, {});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 2_f);
    }

    SECTION("Lambda body allows def and expression separated by semicolons")
    {
        auto result = parse("fn() -> { def x = 1; x }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(value, {});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 1_f);
    }

    SECTION("Lambda body supports semicolons mixed with defs")
    {
        auto result = parse("fn() -> { ; def x = 1 ; x }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(value, {});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 1_f);
    }

    SECTION("Lambda body allows if expression followed by another statement")
    {
        auto result = parse("fn() -> { if true: 1 else: 2; 3 }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(value, {});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 3_f);
    }

    SECTION("Lambda body accepts multiple statements separated by newlines")
    {
        auto result = parse("fn() -> { 1\n2 }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
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
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto out = expr->evaluate(eval_ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 6_f);
    }

    SECTION(
        "Lambda body can contain multiple lambda calls separated by semicolons")
    {
        auto result = parse("fn() -> { fn() -> { 1 }(); fn() -> { 2 }() }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(value, {});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 2_f);
    }

    SECTION("Lambda body supports multiple defs separated by semicolons")
    {
        auto result = parse("fn() -> { def x = 1; def y = 2; y }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(value, {});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 2_f);
    }

    SECTION("Lambda body ending in a definition is rejected")
    {
        CHECK_THROWS_WITH(parse("fn() -> { def x = 1 }"),
                          Catch::Matchers::ContainsSubstring(
                              "A lambda must end in an expression"));
    }

    SECTION("Lambda body allows if expressions")
    {
        auto result = parse("fn() -> { if true: 1 else: 2 }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(value, {});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 1_f);
    }

    SECTION("Non-identifier parameters are rejected")
    {
        CHECK_FALSE(parse("fn(1) -> { 1 }"));
        CHECK_FALSE(parse("fn(\"x\") -> { 1 }"));
    }

    SECTION("Arrow token must be contiguous")
    {
        CHECK_FALSE(parse("fn() - > { 1 }"));
    }

    SECTION("Prefix not before lambda without call is allowed")
    {
        auto result = parse("not fn () -> { null }");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto out = expr->evaluate(eval_ctx);
        REQUIRE(out->is<frst::Bool>());
        CHECK(out->get<frst::Bool>().value() == false);
    }

    SECTION("Whitespace and comments around lambda tokens are allowed")
    {
        auto result = parse("fn(\n # comment\n a ) -> { a } (7)");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto out = expr->evaluate(eval_ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 7_f);
    }

    SECTION("Parameter list allows whitespace and comments between params")
    {
        auto result = parse("fn(a,\n # comment\n b) -> { a + b }(1,2)");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto out = expr->evaluate(eval_ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 3_f);
    }

    SECTION("Lambda can appear as an if condition")
    {
        auto result = parse("if fn() -> { true }(): 1 else: 2");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto out = expr->evaluate(eval_ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 1_f);
    }

    SECTION("Malformed trailing tokens are rejected")
    {
        CHECK_FALSE(parse("fn() -> { 1 } )"));
        CHECK_FALSE(parse("fn() -> { 1 } }"));
        CHECK_FALSE(parse("fn() -> { 1 } extra"));
    }

    SECTION("Parameter list syntax errors are rejected")
    {
        CHECK_FALSE(parse("fn(a,) -> { 1 }"));
        CHECK_FALSE(parse("fn(,a) -> { 1 }"));
        CHECK_FALSE(parse("fn(a b) -> { 1 }"));
        CHECK_FALSE(parse("fn(,) -> { 1 }"));
        CHECK_FALSE(parse("fn(a, ...rest, b) -> { 1 }"));
        CHECK_FALSE(parse("fn(... ) -> { 1 }"));
        CHECK_FALSE(parse("fn(a, ...rest,) -> { 1 }"));
        CHECK_FALSE(parse("fn(, ...rest) -> { 1 }"));
        CHECK_FALSE(parse("fn(...rest, ...more) -> { 1 }"));
        CHECK_FALSE(parse("fn a b -> { 1 }"));
        CHECK_FALSE(parse("fn a, -> { 1 }"));
        CHECK_FALSE(parse("fn ... -> { 1 }"));
        CHECK_FALSE(parse("fn a, ...rest, b -> { 1 }"));
        CHECK_FALSE(parse("fn a, ...rest, -> { 1 }"));
    }

    SECTION("Missing arrow or body is rejected")
    {
        CHECK_FALSE(parse("fn() {}"));
        CHECK_FALSE(parse("fn ->"));
        CHECK_FALSE(parse("fn() ->"));
        CHECK_FALSE(parse("fn() -> {"));
        CHECK_FALSE(parse("fn() -> }"));
    }

    SECTION("Named lambda: single param")
    {
        auto result = parse("fn f(x) -> x");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(value, {frst::Value::create(7_f)});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 7_f);
    }

    SECTION("Named lambda: multiple params")
    {
        auto result = parse("fn f(x, y) -> x + y");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(
            value, {frst::Value::create(3_f), frst::Value::create(4_f)});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 7_f);
    }

    SECTION("Named lambda: no params")
    {
        auto result = parse("fn f() -> 42");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(value, {});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 42_f);
    }

    SECTION("Named lambda: vararg-only")
    {
        auto result = parse("fn f(...rest) -> rest");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(
            value, {frst::Value::create(1_f), frst::Value::create(2_f)});
        REQUIRE(out->is<frst::Array>());
        CHECK(out->raw_get<frst::Array>().size() == 2);
    }

    SECTION("Named lambda: params plus vararg")
    {
        auto result = parse("fn f(x, ...rest) -> rest");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(value, {frst::Value::create(1_f),
                                         frst::Value::create(2_f),
                                         frst::Value::create(3_f)});
        REQUIRE(out->is<frst::Array>());
        CHECK(out->raw_get<frst::Array>().size() == 2);
    }

    SECTION("Named lambda: space between name and paren gives same result")
    {
        auto result_nospace = parse("fn f(x) -> x");
        auto result_space = parse("fn f (x) -> x");
        REQUIRE(result_nospace);
        REQUIRE(result_space);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto v1 = require_expression(result_nospace)->evaluate(eval_ctx);
        auto v2 = require_expression(result_space)->evaluate(eval_ctx);

        auto arg = frst::Value::create(5_f);
        CHECK(call_function(v1, {arg})->get<frst::Int>().value() == 5_f);
        CHECK(call_function(v2, {arg})->get<frst::Int>().value() == 5_f);
    }

    SECTION("Named lambda: self-name enables recursion")
    {
        auto result = parse("fn fact(n) -> if n <= 1: 1 else: n * fact(n - 1)");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(value, {frst::Value::create(5_f)});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 120_f);
    }

    SECTION("Regression: fn(x) -> x is still unnamed")
    {
        auto result = parse("fn(x) -> x");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto closure = std::dynamic_pointer_cast<frst::Closure>(
            value->get<frst::Function>().value());
        REQUIRE(closure);
        CHECK_FALSE(closure->debug_capture_table().has("f"));
    }

    SECTION("Regression: fn x -> x is unnamed with x as param")
    {
        auto result = parse("fn x -> x");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto out = call_function(value, {frst::Value::create(9_f)});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 9_f);

        auto closure = std::dynamic_pointer_cast<frst::Closure>(
            value->get<frst::Function>().value());
        REQUIRE(closure);
        CHECK_FALSE(closure->debug_capture_table().has("x"));
    }

    SECTION("Regression: fn f -> body treats f as a param, not self-name")
    {
        auto result = parse("fn f -> f");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto value = expr->evaluate(eval_ctx);
        auto arg = frst::Value::create(3_f);
        auto out = call_function(value, {arg});
        CHECK(out == arg);
    }

    SECTION("Keyword parameters are rejected")
    {
        CHECK_FALSE(parse("fn(if) -> { 1 }"));
        CHECK_FALSE(parse("fn(true) -> { 1 }"));
        CHECK_FALSE(parse("fn(fn) -> { 1 }"));
        CHECK_FALSE(parse("fn(...if) -> { 1 }"));
    }

    SECTION("Variadic parameter tolerates comments")
    {
        auto result = parse("fn(... # comment\n rest) -> { rest }(9)");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context eval_ctx{.symbols = table};
        auto out = expr->evaluate(eval_ctx);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 1);
        CHECK(arr[0]->get<frst::Int>().value() == 9_f);
    }

    SECTION("Source range for single-param lambda")
    {
        // "fn(x) -> x + 1" → [1:1-1:14]
        auto result = parse("fn(x) -> x + 1");
        REQUIRE(result);
        auto expr = require_expression(result);
        auto range = expr->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 1);
        CHECK(range.end.line == 1);
        CHECK(range.end.column == 14);
    }

    SECTION("Source range for arrow-only lambda")
    {
        // "fn -> 42" → [1:1-1:8]
        auto result = parse("fn -> 42");
        REQUIRE(result);
        auto expr = require_expression(result);
        auto range = expr->source_range();
        CHECK(range.begin.column == 1);
        CHECK(range.end.column == 8);
    }

    SECTION("Source range for block-body lambda")
    {
        // "fn(x) -> { x }" → [1:1-1:14]
        auto result = parse("fn(x) -> { x }");
        REQUIRE(result);
        auto expr = require_expression(result);
        auto range = expr->source_range();
        CHECK(range.begin.column == 1);
        CHECK(range.end.column == 14);
    }

    SECTION("Source range excludes trailing whitespace")
    {
        // "fn -> 1   " → end at column 7, not 10
        auto result = parse("fn -> 1   ");
        REQUIRE(result);
        auto expr = require_expression(result);
        auto range = expr->source_range();
        CHECK(range.begin.column == 1);
        CHECK(range.end.column == 7);
    }

    SECTION("Source range for multiline lambda")
    {
        // "fn(x) ->\n  x + 1" → [1:1-2:5]
        auto result = parse("fn(x) ->\n  x + 1");
        REQUIRE(result);
        auto expr = require_expression(result);
        auto range = expr->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 1);
        CHECK(range.end.line == 2);
        CHECK(range.end.column == 7);
    }

    SECTION("Source range for multiline block lambda")
    {
        // "fn(x) -> {\n  x\n}" → [1:1-3:1]
        auto result = parse("fn(x) -> {\n  x\n}");
        REQUIRE(result);
        auto expr = require_expression(result);
        auto range = expr->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 1);
        CHECK(range.end.line == 3);
        CHECK(range.end.column == 1);
    }
}
