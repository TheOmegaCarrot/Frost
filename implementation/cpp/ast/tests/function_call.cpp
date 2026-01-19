#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>
#include <catch2/trompeloeil.hpp>

#include <frost/ast.hpp>
#include <frost/mock/mock-callable.hpp>
#include <frost/mock/mock-expression.hpp>
#include <frost/mock/mock-symbol-table.hpp>
#include <frost/value.hpp>

#include <algorithm>

using namespace frst;
using namespace frst::ast;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;
using trompeloeil::_;

namespace
{
using Call_List = std::vector<std::vector<Value_Ptr>>;

void record_call(Call_List& calls, std::span<const Value_Ptr> args)
{
    calls.emplace_back(args.begin(), args.end());
}
} // namespace

TEST_CASE("Function Call")
{
    // AI-generated test additions by Codex (GPT-5).
    // Signed: Codex (GPT-5).

    mock::Mock_Symbol_Table syms;

    SECTION("Evaluates function then args left-to-right and passes pointers")
    {
        auto fn_expr = std::make_unique<mock::Mock_Expression>();
        auto arg1 = std::make_unique<mock::Mock_Expression>();
        auto arg2 = std::make_unique<mock::Mock_Expression>();

        auto* fn_ptr = fn_expr.get();
        auto* arg1_ptr = arg1.get();
        auto* arg2_ptr = arg2.get();

        auto callable = mock::Mock_Callable::make();
        auto ret = Value::create(7_f);

        auto v1 = Value::create(1_f);
        auto v2 = Value::create(2_f);
        Call_List calls;

        auto fn_val = Value::create(Function{callable});

        trompeloeil::sequence seq;
        REQUIRE_CALL(*fn_ptr, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(fn_val);
        REQUIRE_CALL(*arg1_ptr, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(v1);
        REQUIRE_CALL(*arg2_ptr, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(v2);
        REQUIRE_CALL(*callable, call(_))
            .LR_SIDE_EFFECT(record_call(calls, _1))
            .LR_WITH(_1.size() == 2 && _1[0] == v1 && _1[1] == v2)
            .RETURN(ret);

        std::vector<Expression::Ptr> args;
        args.push_back(std::move(arg1));
        args.push_back(std::move(arg2));

        ast::Function_Call node{std::move(fn_expr), std::move(args)};
        auto res = node.evaluate(syms);

        CHECK(res == ret);
        REQUIRE(calls.size() == 1);
        CHECK(calls.at(0).size() == 2);
        CHECK(calls.at(0).at(0) == v1);
        CHECK(calls.at(0).at(1) == v2);
    }

    SECTION("Empty argument list calls function")
    {
        auto fn_expr = std::make_unique<mock::Mock_Expression>();
        auto* fn_ptr = fn_expr.get();

        auto callable = mock::Mock_Callable::make();
        auto fn_val = Value::create(Function{callable});
        Call_List calls;

        REQUIRE_CALL(*fn_ptr, evaluate(_)).LR_WITH(&_1 == &syms).RETURN(fn_val);
        REQUIRE_CALL(*callable, call(_))
            .LR_SIDE_EFFECT(record_call(calls, _1))
            .LR_WITH(_1.empty())
            .RETURN(Value::null());

        std::vector<Expression::Ptr> args;
        ast::Function_Call node{std::move(fn_expr), std::move(args)};

        auto res = node.evaluate(syms);
        CHECK(res->is<Null>());
        REQUIRE(calls.size() == 1);
        CHECK(calls.at(0).empty());
    }

    SECTION("Same node can be evaluated multiple times")
    {
        auto fn_expr = std::make_unique<mock::Mock_Expression>();
        auto arg = std::make_unique<mock::Mock_Expression>();

        auto* fn_ptr = fn_expr.get();
        auto* arg_ptr = arg.get();

        auto callable = mock::Mock_Callable::make();
        auto ret = Value::create(9_f);

        auto arg_val = Value::create(3_f);
        Call_List calls;

        auto fn_val = Value::create(Function{callable});

        REQUIRE_CALL(*fn_ptr, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .TIMES(2)
            .RETURN(fn_val);
        REQUIRE_CALL(*arg_ptr, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .TIMES(2)
            .RETURN(arg_val);
        REQUIRE_CALL(*callable, call(_))
            .LR_SIDE_EFFECT(record_call(calls, _1))
            .LR_WITH(_1.size() == 1 && _1[0] == arg_val)
            .RETURN(ret)
            .TIMES(2);

        std::vector<Expression::Ptr> args;
        args.push_back(std::move(arg));

        ast::Function_Call node{std::move(fn_expr), std::move(args)};

        CHECK(node.evaluate(syms) == ret);
        CHECK(node.evaluate(syms) == ret);
        CHECK(calls.size() == 2);
    }

    SECTION("Large argument list preserves order")
    {
        auto fn_expr = std::make_unique<mock::Mock_Expression>();
        auto* fn_ptr = fn_expr.get();

        auto callable = mock::Mock_Callable::make();
        auto ret = Value::create(0_f);

        std::vector<Expression::Ptr> args;
        std::vector<Value_Ptr> expected;
        for (int i = 0; i < 8; ++i)
        {
            auto val = Value::create(static_cast<Int>(i + 1));
            expected.push_back(val);
            args.push_back(std::make_unique<Literal>(val));
        }
        Call_List calls;

        auto fn_val = Value::create(Function{callable});
        REQUIRE_CALL(*fn_ptr, evaluate(_)).LR_WITH(&_1 == &syms).RETURN(fn_val);
        REQUIRE_CALL(*callable, call(_))
            .LR_SIDE_EFFECT(record_call(calls, _1))
            .LR_WITH(std::equal(_1.begin(), _1.end(), expected.begin()))
            .RETURN(ret);

        ast::Function_Call node{std::move(fn_expr), std::move(args)};
        auto res = node.evaluate(syms);

        CHECK(res == ret);
        REQUIRE(calls.size() == 1);
        CHECK(std::equal(calls.at(0).begin(), calls.at(0).end(),
                         expected.begin()));
    }

    SECTION("Non-function callee throws and args are not evaluated")
    {
        auto fn_expr = std::make_unique<mock::Mock_Expression>();
        auto arg = std::make_unique<mock::Mock_Expression>();

        auto* fn_ptr = fn_expr.get();
        auto* arg_ptr = arg.get();

        REQUIRE_CALL(*fn_ptr, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .RETURN(Value::create(1_f));
        FORBID_CALL(*arg_ptr, evaluate(_));

        std::vector<Expression::Ptr> args;
        args.push_back(std::move(arg));

        ast::Function_Call node{std::move(fn_expr), std::move(args)};

        CHECK_THROWS_WITH(node.evaluate(syms),
                          ContainsSubstring("Cannot call value of type Int"));
    }

    SECTION("Function expression error propagates")
    {
        auto fn_expr = std::make_unique<mock::Mock_Expression>();
        auto arg = std::make_unique<mock::Mock_Expression>();

        auto* fn_ptr = fn_expr.get();
        auto* arg_ptr = arg.get();

        REQUIRE_CALL(*fn_ptr, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .THROW(Frost_User_Error{"fn boom"});
        FORBID_CALL(*arg_ptr, evaluate(_));

        std::vector<Expression::Ptr> args;
        args.push_back(std::move(arg));

        ast::Function_Call node{std::move(fn_expr), std::move(args)};

        CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("fn boom"));
    }

    SECTION("Argument evaluation error propagates and call is not invoked")
    {
        auto fn_expr = std::make_unique<mock::Mock_Expression>();
        auto arg1 = std::make_unique<mock::Mock_Expression>();
        auto arg2 = std::make_unique<mock::Mock_Expression>();

        auto* fn_ptr = fn_expr.get();
        auto* arg1_ptr = arg1.get();
        auto* arg2_ptr = arg2.get();

        auto callable = mock::Mock_Callable::make();
        auto fn_val = Value::create(Function{callable});

        REQUIRE_CALL(*fn_ptr, evaluate(_)).LR_WITH(&_1 == &syms).RETURN(fn_val);
        REQUIRE_CALL(*arg1_ptr, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .THROW(Frost_User_Error{"arg boom"});
        FORBID_CALL(*arg2_ptr, evaluate(_));
        FORBID_CALL(*callable, call(_));

        std::vector<Expression::Ptr> args;
        args.push_back(std::move(arg1));
        args.push_back(std::move(arg2));

        ast::Function_Call node{std::move(fn_expr), std::move(args)};

        CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("arg boom"));
    }

    SECTION("Callable error propagates")
    {
        auto fn_expr = std::make_unique<mock::Mock_Expression>();
        auto arg = std::make_unique<mock::Mock_Expression>();

        auto* fn_ptr = fn_expr.get();
        auto* arg_ptr = arg.get();

        auto callable = mock::Mock_Callable::make();
        auto fn_val = Value::create(Function{callable});

        REQUIRE_CALL(*fn_ptr, evaluate(_)).LR_WITH(&_1 == &syms).RETURN(fn_val);
        REQUIRE_CALL(*arg_ptr, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .RETURN(Value::create(1_f));
        REQUIRE_CALL(*callable, call(_)).THROW(Frost_User_Error{"boom"});

        std::vector<Expression::Ptr> args;
        args.push_back(std::move(arg));

        ast::Function_Call node{std::move(fn_expr), std::move(args)};

        CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("boom"));
    }
}
