#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>
#include <catch2/trompeloeil.hpp>

#include <frost/ast.hpp>
#include <frost/mock/mock-expression.hpp>
#include <frost/mock/mock-symbol-table.hpp>
#include <frost/value.hpp>

using namespace frst;
using namespace frst::ast;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;
using trompeloeil::_;

namespace
{
struct RecordingCallable final : Callable
{
    mutable bool called = false;
    mutable bool args_match = false;
    mutable int call_count = 0;
    std::vector<Value_Ptr> expected_args;
    Value_Ptr result;

    Value_Ptr call(const std::vector<Value_Ptr>& args) const override
    {
        called = true;
        ++call_count;
        args_match = (args == expected_args);
        return result ? result : Value::null();
    }

    std::string debug_dump() const override
    {
        return "<recording>";
    }
};

struct ThrowingCallable final : Callable
{
    Value_Ptr call(const std::vector<Value_Ptr>&) const override
    {
        throw Frost_User_Error{"boom"};
    }

    std::string debug_dump() const override
    {
        return "<throwing>";
    }
};
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

        auto callable = std::make_shared<RecordingCallable>();
        auto ret = Value::create(7_f);
        callable->result = ret;

        auto v1 = Value::create(1_f);
        auto v2 = Value::create(2_f);
        callable->expected_args = {v1, v2};

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

        std::vector<Expression::Ptr> args;
        args.push_back(std::move(arg1));
        args.push_back(std::move(arg2));

        ast::Function_Call node{std::move(fn_expr), std::move(args)};
        auto res = node.evaluate(syms);

        CHECK(res == ret);
        CHECK(callable->called);
        CHECK(callable->args_match);
    }

    SECTION("Empty argument list calls function")
    {
        auto fn_expr = std::make_unique<mock::Mock_Expression>();
        auto* fn_ptr = fn_expr.get();

        auto callable = std::make_shared<RecordingCallable>();
        callable->expected_args = {};
        auto fn_val = Value::create(Function{callable});

        REQUIRE_CALL(*fn_ptr, evaluate(_)).LR_WITH(&_1 == &syms).RETURN(fn_val);

        std::vector<Expression::Ptr> args;
        ast::Function_Call node{std::move(fn_expr), std::move(args)};

        auto res = node.evaluate(syms);
        CHECK(res->is<Null>());
        CHECK(callable->called);
        CHECK(callable->args_match);
    }

    SECTION("Same node can be evaluated multiple times")
    {
        auto fn_expr = std::make_unique<mock::Mock_Expression>();
        auto arg = std::make_unique<mock::Mock_Expression>();

        auto* fn_ptr = fn_expr.get();
        auto* arg_ptr = arg.get();

        auto callable = std::make_shared<RecordingCallable>();
        auto ret = Value::create(9_f);
        callable->result = ret;

        auto arg_val = Value::create(3_f);
        callable->expected_args = {arg_val};

        auto fn_val = Value::create(Function{callable});

        REQUIRE_CALL(*fn_ptr, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .TIMES(2)
            .RETURN(fn_val);
        REQUIRE_CALL(*arg_ptr, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .TIMES(2)
            .RETURN(arg_val);

        std::vector<Expression::Ptr> args;
        args.push_back(std::move(arg));

        ast::Function_Call node{std::move(fn_expr), std::move(args)};

        CHECK(node.evaluate(syms) == ret);
        CHECK(node.evaluate(syms) == ret);
        CHECK(callable->call_count == 2);
        CHECK(callable->args_match);
    }

    SECTION("Large argument list preserves order")
    {
        auto fn_expr = std::make_unique<mock::Mock_Expression>();
        auto* fn_ptr = fn_expr.get();

        auto callable = std::make_shared<RecordingCallable>();
        auto ret = Value::create(0_f);
        callable->result = ret;

        std::vector<Expression::Ptr> args;
        std::vector<Value_Ptr> expected;
        for (int i = 0; i < 8; ++i)
        {
            auto val = Value::create(static_cast<Int>(i + 1));
            expected.push_back(val);
            args.push_back(std::make_unique<Literal>(val));
        }
        callable->expected_args = expected;

        auto fn_val = Value::create(Function{callable});
        REQUIRE_CALL(*fn_ptr, evaluate(_)).LR_WITH(&_1 == &syms).RETURN(fn_val);

        ast::Function_Call node{std::move(fn_expr), std::move(args)};
        auto res = node.evaluate(syms);

        CHECK(res == ret);
        CHECK(callable->called);
        CHECK(callable->args_match);
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

        auto callable = std::make_shared<RecordingCallable>();
        auto fn_val = Value::create(Function{callable});

        REQUIRE_CALL(*fn_ptr, evaluate(_)).LR_WITH(&_1 == &syms).RETURN(fn_val);
        REQUIRE_CALL(*arg1_ptr, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .THROW(Frost_User_Error{"arg boom"});
        FORBID_CALL(*arg2_ptr, evaluate(_));

        std::vector<Expression::Ptr> args;
        args.push_back(std::move(arg1));
        args.push_back(std::move(arg2));

        ast::Function_Call node{std::move(fn_expr), std::move(args)};

        CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("arg boom"));
        CHECK_FALSE(callable->called);
    }

    SECTION("Callable error propagates")
    {
        auto fn_expr = std::make_unique<mock::Mock_Expression>();
        auto arg = std::make_unique<mock::Mock_Expression>();

        auto* fn_ptr = fn_expr.get();
        auto* arg_ptr = arg.get();

        auto callable = std::make_shared<ThrowingCallable>();
        auto fn_val = Value::create(Function{callable});

        REQUIRE_CALL(*fn_ptr, evaluate(_)).LR_WITH(&_1 == &syms).RETURN(fn_val);
        REQUIRE_CALL(*arg_ptr, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .RETURN(Value::create(1_f));

        std::vector<Expression::Ptr> args;
        args.push_back(std::move(arg));

        ast::Function_Call node{std::move(fn_expr), std::move(args)};

        CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("boom"));
    }
}
