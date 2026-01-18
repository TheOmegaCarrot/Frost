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
} // namespace

TEST_CASE("Parser If Expressions")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        auto src = lexy::string_input(input);
        return lexy::parse<Expression_Root>(src, lexy::noop);
    };

    SECTION("Basic if/else evaluates")
    {
        auto result = parse("if true: 1 else: 2");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto value = expr->evaluate(table);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 1_f);

        auto result2 = parse("if false: 1 else: 2");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto value2 = expr2->evaluate(table);
        REQUIRE(value2->is<frst::Int>());
        CHECK(value2->get<frst::Int>().value() == 2_f);
    }

    SECTION("If without else returns Null when false")
    {
        auto result = parse("if false: 1");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto value = expr->evaluate(table);
        REQUIRE(value->is<frst::Null>());
    }

    SECTION("If without else returns consequent when true")
    {
        auto result = parse("if true: 1");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto value = expr->evaluate(table);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 1_f);
    }

    SECTION("Elif chains evaluate left-to-right")
    {
        auto result = parse("if false: 1 elif true: 2 else: 3");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto value = expr->evaluate(table);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 2_f);
    }

    SECTION("Elif chain without else can yield Null")
    {
        auto result = parse("if false: 1 elif false: 2");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto value = expr->evaluate(table);
        REQUIRE(value->is<frst::Null>());
    }

    SECTION("Nested if expressions associate correctly")
    {
        auto result = parse("if true: if false: 1 else: 2 else: 3");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto value = expr->evaluate(table);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 2_f);
    }

    SECTION("Longer elif chains evaluate correctly")
    {
        auto result = parse("if false: 1 elif false: 2 elif true: 3 else: 4");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto value = expr->evaluate(table);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 3_f);
    }

    SECTION("If expression binds as an atom in binary expressions")
    {
        auto result = parse("1 + if true: 2 else: 3");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto value = expr->evaluate(table);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 3_f);

        auto result2 = parse("if false: 1 else: 2 + 3");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto value2 = expr2->evaluate(table);
        REQUIRE(value2->is<frst::Int>());
        CHECK(value2->get<frst::Int>().value() == 5_f);
    }

    SECTION("If branches preserve standard precedence")
    {
        auto result = parse("if false: 1 else: 2 + 3 * 4");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto value = expr->evaluate(table);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 14_f);

        auto result2 = parse("1 + if true: 2 else: 3 * 4");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto value2 = expr2->evaluate(table);
        REQUIRE(value2->is<frst::Int>());
        CHECK(value2->get<frst::Int>().value() == 3_f);
    }

    SECTION("If branches can contain postfix expressions")
    {
        frst::Symbol_Table table;

        auto arr_val = frst::Value::create(frst::Array{
            frst::Value::create(10_f),
            frst::Value::create(20_f),
        });
        table.define("arr", arr_val);

        frst::Map obj;
        obj.emplace(frst::Value::create(std::string{"key"}),
                    frst::Value::create(7_f));
        table.define("obj", frst::Value::create(std::move(obj)));

        auto result = parse("if true: arr[0] else: obj.key");
        REQUIRE(result);
        auto expr = require_expression(result);
        auto value = expr->evaluate(table);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 10_f);
    }

    SECTION("If expression can be parenthesized and then called")
    {
        frst::Symbol_Table table;

        struct ConstantCallable final : frst::Callable
        {
            frst::Value_Ptr value;
            frst::Value_Ptr call(
                std::span<const frst::Value_Ptr>) const override
            {
                return value ? value : frst::Value::null();
            }

            std::string debug_dump() const override
            {
                return "<constant>";
            }
        };

        auto fn_a = std::make_shared<ConstantCallable>();
        fn_a->value = frst::Value::create(10_f);
        table.define("a", frst::Value::create(frst::Function{fn_a}));

        auto fn_b = std::make_shared<ConstantCallable>();
        fn_b->value = frst::Value::create(20_f);
        table.define("b", frst::Value::create(frst::Function{fn_b}));

        auto result = parse("(if true: a else: b)(1)");
        REQUIRE(result);
        auto expr = require_expression(result);
        auto value = expr->evaluate(table);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 10_f);
    }

    SECTION("If condition uses boolean coercion")
    {
        auto result = parse("if 0: 1 else: 2");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto value = expr->evaluate(table);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 1_f);

        auto result2 = parse("if null: 1 else: 2");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto value2 = expr2->evaluate(table);
        REQUIRE(value2->is<frst::Int>());
        CHECK(value2->get<frst::Int>().value() == 2_f);
    }

    SECTION("Whitespace and comments around colons and keywords")
    {
        auto result = parse("if true : 1 else : 2");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto value = expr->evaluate(table);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 1_f);

        auto result2 = parse("if true: # c\n1 else: 2");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto value2 = expr2->evaluate(table);
        REQUIRE(value2->is<frst::Int>());
        CHECK(value2->get<frst::Int>().value() == 1_f);

        auto result3 = parse("if false:\n1\nelse:\n2");
        REQUIRE(result3);
        auto expr3 = require_expression(result3);
        auto value3 = expr3->evaluate(table);
        REQUIRE(value3->is<frst::Int>());
        CHECK(value3->get<frst::Int>().value() == 2_f);
    }

    SECTION("Keyword prefixes in branches are treated as identifiers")
    {
        frst::Symbol_Table table;

        auto elifx = frst::Value::create(7_f);
        auto elsey = frst::Value::create(9_f);
        table.define("elifx", elifx);
        table.define("elsey", elsey);

        auto result = parse("if true: elifx else: elsey");
        REQUIRE(result);
        auto expr = require_expression(result);
        auto value = expr->evaluate(table);
        CHECK(value == elifx);
    }

    SECTION("Invalid if syntax fails to parse")
    {
        const std::string_view cases[] = {
            "if true 1",
            "if true: 1 else 2",
            "if true: 1 else:",
            "if true: 1 elif: 2",
            "if true: 1 elif false 2",
            "if true: 1 else: 2 else: 3",
            "if true: 1 else: 2 elif false: 3",
            "if true:",
            "if true: 1 elif false:",
            "elif true: 1",
            "else: 1",
            "if : 1",
            "if true: 1 else: 2 3",
        };

        for (const auto& input : cases)
        {
            CHECK_FALSE(parse(input));
        }
    }
}
