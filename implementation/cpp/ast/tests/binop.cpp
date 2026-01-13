#include <catch2/catch_test_macros.hpp>

#include <catch2/matchers/catch_matchers_string.hpp>
#include <catch2/trompeloeil.hpp>

#include <frost/mock/mock-expression.hpp>
#include <frost/mock/mock-symbol-table.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/ast.hpp>

using namespace frst;
using namespace std::literals;

using Catch::Matchers::ContainsSubstring;
using trompeloeil::_;

TEST_CASE("Numeric Binary ops")
{
    auto lhs = std::make_unique<mock::Mock_Expression>();
    auto rhs = std::make_unique<mock::Mock_Expression>();
    mock::Mock_Symbol_Table syms;

    auto lhs_val = Value::create(42_f);
    auto rhs_val = Value::create(10_f);

    for (const std::string op : {"+", "-", "*", "/"})
    {
        DYNAMIC_SECTION("Operator " << op)
        {
            trompeloeil::sequence seq;

            REQUIRE_CALL(*lhs, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(lhs_val);

            REQUIRE_CALL(*rhs, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(rhs_val);

            ast::Binop node(std::move(lhs), op, std::move(rhs));

            auto res = node.evaluate(syms);

            if (op == "+")
                CHECK(res->get<Int>() == 42_f + 10_f);
            if (op == "-")
                CHECK(res->get<Int>() == 42_f - 10_f);
            if (op == "*")
                CHECK(res->get<Int>() == 42_f * 10_f);
            if (op == "/")
                CHECK(res->get<Int>() == 42_f / 10_f);
        }
    }
}

TEST_CASE("Binop Short-Circuit")
{
    // AI-generated test additions by Codex (GPT-5).
    mock::Mock_Symbol_Table syms;

    SECTION("and short-circuits on falsy lhs")
    {
        auto lhs = mock::Mock_Expression::make();
        auto rhs = mock::Mock_Expression::make();

        auto lhs_val = Value::create(false);

        REQUIRE_CALL(*lhs, evaluate(_)).LR_WITH(&_1 == &syms).RETURN(lhs_val);
        FORBID_CALL(*rhs, evaluate(_));

        ast::Binop node(std::move(lhs), "and", std::move(rhs));

        auto res = node.evaluate(syms);
        CHECK(res == lhs_val);
    }

    SECTION("and evaluates rhs when lhs is truthy")
    {
        auto lhs = mock::Mock_Expression::make();
        auto rhs = mock::Mock_Expression::make();

        auto lhs_val = Value::create(1_f);
        auto rhs_val = Value::create("rhs"s);

        trompeloeil::sequence seq;
        REQUIRE_CALL(*lhs, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(lhs_val);
        REQUIRE_CALL(*rhs, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(rhs_val);

        ast::Binop node(std::move(lhs), "and", std::move(rhs));

        auto res = node.evaluate(syms);
        CHECK(res == rhs_val);
    }

    SECTION("or short-circuits on truthy lhs")
    {
        auto lhs = mock::Mock_Expression::make();
        auto rhs = mock::Mock_Expression::make();

        auto lhs_val = Value::create(1_f);

        REQUIRE_CALL(*lhs, evaluate(_)).LR_WITH(&_1 == &syms).RETURN(lhs_val);
        FORBID_CALL(*rhs, evaluate(_));

        ast::Binop node(std::move(lhs), "or", std::move(rhs));

        auto res = node.evaluate(syms);
        CHECK(res == lhs_val);
    }

    SECTION("or evaluates rhs when lhs is falsy")
    {
        auto lhs = mock::Mock_Expression::make();
        auto rhs = mock::Mock_Expression::make();

        auto lhs_val = Value::create(false);
        auto rhs_val = Value::create("rhs"s);

        trompeloeil::sequence seq;
        REQUIRE_CALL(*lhs, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(lhs_val);
        REQUIRE_CALL(*rhs, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(rhs_val);

        ast::Binop node(std::move(lhs), "or", std::move(rhs));

        auto res = node.evaluate(syms);
        CHECK(res == rhs_val);
    }
}

TEST_CASE("Comparison Binary ops")
{
    // AI-generated test additions by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    mock::Mock_Symbol_Table syms;

    auto eval_binop = [&](const Value_Ptr& lhs_val, const std::string& op,
                          const Value_Ptr& rhs_val) {
        auto lhs = mock::Mock_Expression::make();
        auto rhs = mock::Mock_Expression::make();

        trompeloeil::sequence seq;
        REQUIRE_CALL(*lhs, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(lhs_val);
        REQUIRE_CALL(*rhs, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(rhs_val);

        ast::Binop node(std::move(lhs), op, std::move(rhs));
        return node.evaluate(syms);
    };

    SECTION("Equality and inequality")
    {
        auto v_null = Value::null();
        auto v_int = Value::create(42_f);
        auto v_int_same = Value::create(42_f);
        auto v_int_other = Value::create(7_f);
        auto v_float = Value::create(42.0);
        auto v_bool = Value::create(true);
        auto v_bool_same = Value::create(true);
        auto v_string = Value::create("hello"s);
        auto v_string_same = Value::create("hello"s);

        CHECK(eval_binop(v_null, "==", v_null)->get<Bool>().value());
        CHECK(eval_binop(v_int, "==", v_int_same)->get<Bool>().value());
        CHECK_FALSE(eval_binop(v_int, "==", v_int_other)->get<Bool>().value());
        CHECK(eval_binop(v_int, "!=", v_int_other)->get<Bool>().value());

        CHECK_FALSE(eval_binop(v_int, "==", v_float)->get<Bool>().value());
        CHECK(eval_binop(v_int, "!=", v_float)->get<Bool>().value());

        CHECK(eval_binop(v_bool, "==", v_bool_same)->get<Bool>().value());
        CHECK(eval_binop(v_string, "==", v_string_same)->get<Bool>().value());

        auto arr1 = Value::create(frst::Array{Value::create(1_f)});
        auto arr2 = Value::create(frst::Array{Value::create(1_f)});
        CHECK(eval_binop(arr1, "==", arr1)->get<Bool>().value());
        CHECK_FALSE(eval_binop(arr1, "==", arr2)->get<Bool>().value());
        CHECK(eval_binop(arr1, "!=", arr2)->get<Bool>().value());

        auto map1 = Value::create(frst::Map{
            {Value::create("k"s), Value::create(1_f)},
        });
        auto map2 = Value::create(frst::Map{
            {Value::create("k"s), Value::create(1_f)},
        });
        CHECK(eval_binop(map1, "==", map1)->get<Bool>().value());
        CHECK_FALSE(eval_binop(map1, "==", map2)->get<Bool>().value());
        CHECK(eval_binop(map1, "!=", map2)->get<Bool>().value());

        struct Dummy final : Callable
        {
            Value_Ptr call(const std::vector<Value_Ptr>&) const override
            {
                return Value::null();
            }
            std::string debug_dump() const override
            {
                return "<dummy>";
            }
        };

        auto fn1 = Value::create(Function{std::make_shared<Dummy>()});
        auto fn2 = Value::create(Function{std::make_shared<Dummy>()});
        CHECK(eval_binop(fn1, "==", fn1)->get<Bool>().value());
        CHECK_FALSE(eval_binop(fn1, "==", fn2)->get<Bool>().value());
        CHECK(eval_binop(fn1, "!=", fn2)->get<Bool>().value());
    }

    SECTION("Ordering comparisons")
    {
        const std::vector<std::string> ops{"<", ">", "<=", ">="};

        for (const auto& op : ops)
        {
            DYNAMIC_SECTION("Numeric op " << op)
            {
                auto lhs = Value::create(2_f);
                auto rhs = Value::create(3.5);

                auto res = eval_binop(lhs, op, rhs);
                REQUIRE(res->is<Bool>());

                if (op == "<")
                    CHECK(res->get<Bool>().value());
                if (op == ">")
                    CHECK_FALSE(res->get<Bool>().value());
                if (op == "<=")
                    CHECK(res->get<Bool>().value());
                if (op == ">=")
                    CHECK_FALSE(res->get<Bool>().value());
            }

            DYNAMIC_SECTION("String op " << op)
            {
                auto lhs = Value::create("apple"s);
                auto rhs = Value::create("banana"s);

                auto res = eval_binop(lhs, op, rhs);
                REQUIRE(res->is<Bool>());

                if (op == "<")
                    CHECK(res->get<Bool>().value());
                if (op == ">")
                    CHECK_FALSE(res->get<Bool>().value());
                if (op == "<=")
                    CHECK(res->get<Bool>().value());
                if (op == ">=")
                    CHECK_FALSE(res->get<Bool>().value());
            }

            DYNAMIC_SECTION("Type error " << op)
            {
                auto lhs = Value::create(true);
                auto rhs = Value::create(false);

                try
                {
                    eval_binop(lhs, op, rhs);
                    FAIL("Expected type error");
                }
                catch (const Frost_Error& err)
                {
                    const std::string msg = err.what();
                    CHECK_THAT(msg, ContainsSubstring("Cannot compare"));
                    CHECK_THAT(msg, ContainsSubstring(op));
                    CHECK_THAT(msg, ContainsSubstring("Bool"));
                }
            }
        }
    }
}
