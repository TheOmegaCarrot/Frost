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

struct RecordingCallable final : frst::Callable
{
    mutable std::vector<frst::Value_Ptr> received;
    frst::Value_Ptr result;

    frst::Value_Ptr call(
        const std::vector<frst::Value_Ptr>& args) const override
    {
        received = args;
        return result ? result : frst::Value::null();
    }

    std::string debug_dump() const override
    {
        return "<recording>";
    }
};

struct IdentityCallable final : frst::Callable
{
    frst::Value_Ptr call(
        const std::vector<frst::Value_Ptr>& args) const override
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

struct ArgsArrayCallable final : frst::Callable
{
    frst::Value_Ptr call(
        const std::vector<frst::Value_Ptr>& args) const override
    {
        frst::Array arr;
        arr.reserve(args.size());
        for (const auto& arg : args)
        {
            arr.push_back(arg);
        }
        return frst::Value::create(std::move(arr));
    }

    std::string debug_dump() const override
    {
        return "<args-array>";
    }
};
} // namespace

TEST_CASE("Parser UFCS Expressions")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        auto src = lexy::string_input(input);
        return lexy::parse<Expression_Root>(src, lexy::noop);
    };

    SECTION("UFCS inserts the left-hand side as the first argument")
    {
        frst::Symbol_Table table;
        auto a_val = frst::Value::create(10_f);
        table.define("a", a_val);

        auto callable = std::make_shared<RecordingCallable>();
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto result = parse("a @ f(1, 2)");
        REQUIRE(result);
        auto expr = require_expression(result);
        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Null>());

        REQUIRE(callable->received.size() == 3);
        CHECK(callable->received[0] == a_val);
        CHECK(callable->received[1]->get<frst::Int>().value() == 1_f);
        CHECK(callable->received[2]->get<frst::Int>().value() == 2_f);
    }

    SECTION("UFCS supports empty argument lists")
    {
        frst::Symbol_Table table;
        auto a_val = frst::Value::create(5_f);
        table.define("a", a_val);

        auto callable = std::make_shared<RecordingCallable>();
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto result = parse("a @ f()");
        REQUIRE(result);
        auto expr = require_expression(result);
        (void)expr->evaluate(table);

        REQUIRE(callable->received.size() == 1);
        CHECK(callable->received[0] == a_val);
    }

    SECTION("UFCS works with dot and index callees")
    {
        frst::Symbol_Table table;
        auto a_val = frst::Value::create(7_f);
        table.define("a", a_val);

        auto callable_dot = std::make_shared<RecordingCallable>();
        frst::Map obj;
        obj.emplace(frst::Value::create(std::string{"m"}),
                    frst::Value::create(frst::Function{callable_dot}));
        table.define("obj", frst::Value::create(std::move(obj)));

        auto callable_index = std::make_shared<RecordingCallable>();
        frst::Array arr;
        arr.push_back(frst::Value::create(frst::Function{callable_index}));
        table.define("arr", frst::Value::create(std::move(arr)));

        auto result1 = parse("a @ obj.m()");
        REQUIRE(result1);
        auto expr1 = require_expression(result1);
        (void)expr1->evaluate(table);

        auto result2 = parse("a @ arr[0]()");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        (void)expr2->evaluate(table);

        REQUIRE(callable_dot->received.size() == 1);
        CHECK(callable_dot->received[0] == a_val);
        REQUIRE(callable_index->received.size() == 1);
        CHECK(callable_index->received[0] == a_val);
    }

    SECTION("UFCS binds more tightly than unary and binary operators")
    {
        frst::Symbol_Table table;
        auto callable = std::make_shared<IdentityCallable>();
        table.define("id", frst::Value::create(frst::Function{callable}));

        auto result1 = parse("1 + 2 @ id()");
        REQUIRE(result1);
        auto expr1 = require_expression(result1);
        auto out1 = expr1->evaluate(table);
        REQUIRE(out1->is<frst::Int>());
        CHECK(out1->get<frst::Int>().value() == 3_f);

        auto result2 = parse("not false @ id()");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto out2 = expr2->evaluate(table);
        REQUIRE(out2->is<frst::Bool>());
        CHECK(out2->get<frst::Bool>().value() == true);
    }

    SECTION("UFCS accepts complex left-hand sides")
    {
        frst::Symbol_Table table;
        frst::Map obj;
        frst::Array arr;
        arr.push_back(frst::Value::create(4_f));
        obj.emplace(frst::Value::create(std::string{"arr"}),
                    frst::Value::create(std::move(arr)));
        table.define("obj", frst::Value::create(std::move(obj)));
        table.define("b", frst::Value::create(6_f));

        auto callable = std::make_shared<IdentityCallable>();
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto result1 = parse("obj.arr[0] @ f()");
        REQUIRE(result1);
        auto expr1 = require_expression(result1);
        auto out1 = expr1->evaluate(table);
        REQUIRE(out1->is<frst::Int>());
        CHECK(out1->get<frst::Int>().value() == 4_f);

        auto result2 = parse("(1 + b) @ f()");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto out2 = expr2->evaluate(table);
        REQUIRE(out2->is<frst::Int>());
        CHECK(out2->get<frst::Int>().value() == 7_f);
    }

    SECTION("UFCS is left-associative")
    {
        frst::Symbol_Table table;
        auto a_val = frst::Value::create(1_f);
        table.define("a", a_val);

        auto callable_f = std::make_shared<RecordingCallable>();
        callable_f->result = frst::Value::create(7_f);
        table.define("f", frst::Value::create(frst::Function{callable_f}));

        auto callable_g = std::make_shared<RecordingCallable>();
        table.define("g", frst::Value::create(frst::Function{callable_g}));

        auto result = parse("a @ f() @ g()");
        REQUIRE(result);
        auto expr = require_expression(result);
        (void)expr->evaluate(table);

        REQUIRE(callable_f->received.size() == 1);
        CHECK(callable_f->received[0] == a_val);
        REQUIRE(callable_g->received.size() == 1);
        REQUIRE(callable_g->received[0]->is<frst::Int>());
        CHECK(callable_g->received[0]->get<frst::Int>().value() == 7_f);
    }

    SECTION("UFCS applies before unary operators on nontrivial expressions")
    {
        frst::Symbol_Table table;
        auto a_val = frst::Value::create(5_f);
        table.define("a", a_val);

        auto callable = std::make_shared<IdentityCallable>();
        table.define("id", frst::Value::create(frst::Function{callable}));

        auto result1 = parse("-a @ id()");
        REQUIRE(result1);
        auto expr1 = require_expression(result1);
        auto out1 = expr1->evaluate(table);
        REQUIRE(out1->is<frst::Int>());
        CHECK(out1->get<frst::Int>().value() == -5_f);

        auto result2 = parse("not a @ id()");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto out2 = expr2->evaluate(table);
        REQUIRE(out2->is<frst::Bool>());
        CHECK(out2->get<frst::Bool>().value() == false);
    }

    SECTION("UFCS allows whitespace and newlines around '@'")
    {
        frst::Symbol_Table table;
        auto a_val = frst::Value::create(3_f);
        table.define("a", a_val);

        auto callable = std::make_shared<RecordingCallable>();
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto result = parse("a\n @  f( )");
        REQUIRE(result);
        auto expr = require_expression(result);
        (void)expr->evaluate(table);

        REQUIRE(callable->received.size() == 1);
        CHECK(callable->received[0] == a_val);
    }

    SECTION("UFCS supports whitespace inside callee chains")
    {
        frst::Symbol_Table table;
        auto a_val = frst::Value::create(8_f);
        table.define("a", a_val);

        auto callable = std::make_shared<RecordingCallable>();
        frst::Map obj;
        obj.emplace(frst::Value::create(std::string{"m"}),
                    frst::Value::create(frst::Function{callable}));
        table.define("obj", frst::Value::create(std::move(obj)));

        auto result = parse("a @ obj .\n m ( )");
        REQUIRE(result);
        auto expr = require_expression(result);
        (void)expr->evaluate(table);

        REQUIRE(callable->received.size() == 1);
        CHECK(callable->received[0] == a_val);
    }

    SECTION("UFCS requires a call expression on the right-hand side")
    {
        CHECK_FALSE(parse("a @ f"));
        CHECK_FALSE(parse("a @ obj.key"));
        CHECK_FALSE(parse("a @ b @ f()"));
        CHECK_FALSE(parse("a @ map [1] with f"));
    }

    SECTION("UFCS rejects malformed call arguments")
    {
        CHECK_FALSE(parse("a @ f("));
        CHECK_FALSE(parse("a @ f(,)"));
        CHECK_FALSE(parse("a @ f(1,)"));
        CHECK_FALSE(parse("a @ f(1 2)"));
    }

    SECTION("UFCS stops at the call and allows postfix on the result")
    {
        frst::Symbol_Table table;
        auto a_val = frst::Value::create(9_f);
        table.define("a", a_val);

        auto callable = std::make_shared<ArgsArrayCallable>();
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto result = parse("a @ f(1)[0]");
        REQUIRE(result);
        auto expr = require_expression(result);
        auto out = expr->evaluate(table);
        CHECK(out == a_val);
    }

    SECTION("UFCS binds tighter than a subsequent call")
    {
        frst::Symbol_Table table;
        auto a_val = frst::Value::create(1_f);
        auto b_val = frst::Value::create(2_f);
        table.define("a", a_val);
        table.define("b", b_val);

        auto g_callable = std::make_shared<RecordingCallable>();
        auto f_callable = std::make_shared<RecordingCallable>();
        f_callable->result = frst::Value::create(frst::Function{g_callable});
        table.define("f", frst::Value::create(frst::Function{f_callable}));

        auto result = parse("a @ f()(b)");
        REQUIRE(result);
        auto expr = require_expression(result);
        (void)expr->evaluate(table);

        REQUIRE(f_callable->received.size() == 1);
        CHECK(f_callable->received[0] == a_val);
        REQUIRE(g_callable->received.size() == 1);
        CHECK(g_callable->received[0] == b_val);
    }

    SECTION("UFCS with dot call followed by indexing")
    {
        frst::Symbol_Table table;
        auto a_val = frst::Value::create(2_f);
        table.define("a", a_val);

        auto callable = std::make_shared<ArgsArrayCallable>();
        frst::Map obj;
        obj.emplace(frst::Value::create(std::string{"m"}),
                    frst::Value::create(frst::Function{callable}));
        table.define("obj", frst::Value::create(std::move(obj)));

        auto result = parse("a @ obj.m()[0]");
        REQUIRE(result);
        auto expr = require_expression(result);
        auto out = expr->evaluate(table);
        CHECK(out == a_val);
    }

    SECTION("UFCS accepts parenthesized lambda calls on the right-hand side")
    {
        auto result = parse("a @ (fn (x) -> { x })(1)");
        REQUIRE(result);
    }
}
