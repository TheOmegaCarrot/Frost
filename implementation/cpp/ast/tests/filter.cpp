#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>
#include <catch2/trompeloeil.hpp>

#include <frost/mock/mock-callable.hpp>
#include <frost/mock/mock-expression.hpp>
#include <frost/mock/mock-symbol-table.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/ast.hpp>
#include <frost/symbol-table.hpp>

using namespace frst;
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

TEST_CASE("Filter")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    mock::Mock_Symbol_Table syms;
    auto structure_expr = mock::Mock_Expression::make();
    auto operation_expr = mock::Mock_Expression::make();

    SECTION("Array")
    {
        SECTION("Empty array returns empty array; predicate not called")
        {
            auto empty = Value::create(Array{});
            auto pred = mock::Mock_Callable::make();
            auto pred_val = Value::create(Function{pred});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(empty);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(pred_val);
            FORBID_CALL(*pred, call(_));

            ast::Filter node{std::move(structure_expr),
                             std::move(operation_expr)};

            auto res = node.evaluate(syms);
            REQUIRE(res->is<Array>());
            auto out = res->get<Array>().value();
            CHECK(out.empty());
            CHECK(calls.empty());
        }

        SECTION("Keeps elements with truthy predicate; preserves order")
        {
            auto v1 = Value::create(1_f);
            auto v2 = Value::create(2_f);
            auto v3 = Value::create(3_f);
            auto array_val = Value::create(Array{v1, v2, v3});

            auto pred = mock::Mock_Callable::make();
            auto pred_val = Value::create(Function{pred});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(array_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(pred_val);
            REQUIRE_CALL(*pred, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .LR_WITH(_1.size() == 1 && _1[0] == v1)
                .IN_SEQUENCE(seq)
                .RETURN(Value::create(true));
            REQUIRE_CALL(*pred, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .LR_WITH(_1.size() == 1 && _1[0] == v2)
                .IN_SEQUENCE(seq)
                .RETURN(Value::create(false));
            REQUIRE_CALL(*pred, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .LR_WITH(_1.size() == 1 && _1[0] == v3)
                .IN_SEQUENCE(seq)
                .RETURN(Value::create(true));

            ast::Filter node{std::move(structure_expr),
                             std::move(operation_expr)};

            auto res = node.evaluate(syms);
            REQUIRE(res->is<Array>());
            auto out = res->get<Array>().value();
            REQUIRE(out.size() == 2);
            CHECK(out.at(0) == v1);
            CHECK(out.at(1) == v3);

            CHECK(calls.size() == 3);
            bool seen_v1 = false;
            bool seen_v2 = false;
            bool seen_v3 = false;
            for (const auto& call : calls)
            {
                REQUIRE(call.size() == 1);
                if (call.at(0) == v1)
                    seen_v1 = true;
                if (call.at(0) == v2)
                    seen_v2 = true;
                if (call.at(0) == v3)
                    seen_v3 = true;
            }
            CHECK(seen_v1);
            CHECK(seen_v2);
            CHECK(seen_v3);
        }

        SECTION("Truthiness uses usual coercion rules")
        {
            auto v1 = Value::create(1_f);
            auto v2 = Value::create(2_f);
            auto v3 = Value::create(3_f);
            auto array_val = Value::create(Array{v1, v2, v3});

            auto pred = mock::Mock_Callable::make();
            auto pred_val = Value::create(Function{pred});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(array_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(pred_val);
            REQUIRE_CALL(*pred, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .LR_WITH(_1.size() == 1 && _1[0] == v1)
                .IN_SEQUENCE(seq)
                .RETURN(Value::null());
            REQUIRE_CALL(*pred, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .LR_WITH(_1.size() == 1 && _1[0] == v2)
                .IN_SEQUENCE(seq)
                .RETURN(Value::create(false));
            REQUIRE_CALL(*pred, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .LR_WITH(_1.size() == 1 && _1[0] == v3)
                .IN_SEQUENCE(seq)
                .RETURN(Value::create(123_f));

            ast::Filter node{std::move(structure_expr),
                             std::move(operation_expr)};

            auto res = node.evaluate(syms);
            REQUIRE(res->is<Array>());
            auto out = res->get<Array>().value();
            REQUIRE(out.size() == 1);
            CHECK(out.at(0) == v3);
            CHECK(calls.size() == 3);
        }

        SECTION("Predicate error propagates and stops")
        {
            auto v1 = Value::create(1_f);
            auto v2 = Value::create(2_f);
            auto v3 = Value::create(3_f);
            auto array_val = Value::create(Array{v1, v2, v3});

            auto pred = mock::Mock_Callable::make();
            auto pred_val = Value::create(Function{pred});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(array_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(pred_val);
            REQUIRE_CALL(*pred, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .LR_WITH(_1.size() == 1 && _1[0] == v1)
                .IN_SEQUENCE(seq)
                .RETURN(Value::create(true));
            REQUIRE_CALL(*pred, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .LR_WITH(_1.size() == 1 && _1[0] == v2)
                .IN_SEQUENCE(seq)
                .THROW(Frost_Recoverable_Error{"kaboom"});

            ast::Filter node{std::move(structure_expr),
                             std::move(operation_expr)};

            CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("kaboom"));
            CHECK(calls.size() == 2);
            bool seen_v3 = false;
            for (const auto& call : calls)
            {
                if (call.size() == 1 && call.at(0) == v3)
                    seen_v3 = true;
            }
            CHECK_FALSE(seen_v3);
        }
    }

    SECTION("Map")
    {
        SECTION("Empty map returns empty map; predicate not called")
        {
            auto empty = Value::create(Map{});
            auto pred = mock::Mock_Callable::make();
            auto pred_val = Value::create(Function{pred});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(empty);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(pred_val);
            FORBID_CALL(*pred, call(_));

            ast::Filter node{std::move(structure_expr),
                             std::move(operation_expr)};

            auto res = node.evaluate(syms);
            REQUIRE(res->is<Map>());
            auto out = res->get<Map>().value();
            CHECK(out.empty());
            CHECK(calls.empty());
        }

        SECTION("Keeps entries with truthy predicate")
        {
            auto k1 = Value::create("k1"s);
            auto v1 = Value::create(1_f);
            auto k2 = Value::create("k2"s);
            auto v2 = Value::create(2_f);
            auto map_val = Value::create(Map{{k1, v1}, {k2, v2}});

            auto pred = mock::Mock_Callable::make();
            auto pred_val = Value::create(Function{pred});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(map_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(pred_val);
            REQUIRE_CALL(*pred, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .LR_WITH(_1.size() == 2 && _1[0] == k1 && _1[1] == v1)
                .RETURN(Value::create(true));
            REQUIRE_CALL(*pred, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .LR_WITH(_1.size() == 2 && _1[0] == k2 && _1[1] == v2)
                .RETURN(Value::create(false));

            ast::Filter node{std::move(structure_expr),
                             std::move(operation_expr)};

            auto res = node.evaluate(syms);
            REQUIRE(res->is<Map>());
            auto out = res->get<Map>().value();
            REQUIRE(out.size() == 1);
            CHECK(out.contains(k1));
            CHECK_FALSE(out.contains(k2));
            CHECK(out.at(k1) == v1);
            CHECK(out.begin()->first == k1);

            CHECK(calls.size() == 2);
            bool seen_k1 = false;
            bool seen_k2 = false;
            for (const auto& call : calls)
            {
                REQUIRE(call.size() == 2);
                if (call.at(0) == k1 && call.at(1) == v1)
                    seen_k1 = true;
                if (call.at(0) == k2 && call.at(1) == v2)
                    seen_k2 = true;
            }
            CHECK(seen_k1);
            CHECK(seen_k2);
        }

        SECTION("Truthiness uses usual coercion rules")
        {
            auto k1 = Value::create("k1"s);
            auto v1 = Value::create(1_f);
            auto k2 = Value::create("k2"s);
            auto v2 = Value::create(2_f);
            auto map_val = Value::create(Map{{k1, v1}, {k2, v2}});

            auto pred = mock::Mock_Callable::make();
            auto pred_val = Value::create(Function{pred});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(map_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(pred_val);
            REQUIRE_CALL(*pred, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .LR_WITH(_1.size() == 2 && _1[0] == k1 && _1[1] == v1)
                .RETURN(Value::null());
            REQUIRE_CALL(*pred, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .LR_WITH(_1.size() == 2 && _1[0] == k2 && _1[1] == v2)
                .RETURN(Value::create(42_f));

            ast::Filter node{std::move(structure_expr),
                             std::move(operation_expr)};

            auto res = node.evaluate(syms);
            REQUIRE(res->is<Map>());
            auto out = res->get<Map>().value();
            REQUIRE(out.size() == 1);
            CHECK(out.contains(k2));
            CHECK_FALSE(out.contains(k1));
            CHECK(out.begin()->first == k2);
            CHECK(calls.size() == 2);
        }

        SECTION("Predicate error propagates and stops")
        {
            auto k1 = Value::create("k1"s);
            auto v1 = Value::create(1_f);
            auto k2 = Value::create("k2"s);
            auto v2 = Value::create(2_f);
            auto map_val = Value::create(Map{{k1, v1}, {k2, v2}});

            auto pred = mock::Mock_Callable::make();
            auto pred_val = Value::create(Function{pred});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(map_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(pred_val);
            REQUIRE_CALL(*pred, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .THROW(Frost_Recoverable_Error{"kaboom"});

            ast::Filter node{std::move(structure_expr),
                             std::move(operation_expr)};

            CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("kaboom"));
            REQUIRE(calls.size() == 1);
            REQUIRE(calls.at(0).size() == 2);
        }
    }

    SECTION("Type errors")
    {
        SECTION("Non-structured input errors and skips predicate evaluation")
        {
            auto bad_val = Value::create(123_f);

            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .RETURN(bad_val);
            FORBID_CALL(*operation_expr, evaluate(_));

            ast::Filter node{std::move(structure_expr),
                             std::move(operation_expr)};

            CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("Int"));
        }

        SECTION("Non-function predicate errors and includes type name")
        {
            auto array_val = Value::create(Array{});
            auto bad_pred = Value::create("nope"s);

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(array_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(bad_pred);

            ast::Filter node{std::move(structure_expr),
                             std::move(operation_expr)};

            CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("String"));
        }
    }
}
