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

TEST_CASE("Reduce Array")
{
    // AI-generated test skeleton by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    mock::Mock_Symbol_Table syms;
    auto structure_expr = mock::Mock_Expression::make();
    auto operation_expr = mock::Mock_Expression::make();
    auto init_expr = mock::Mock_Expression::make();

    SECTION("Success cases")
    {
        // Frost: reduce [] with fn (acc, elem) -> { acc + elem }
        SECTION("Empty array without init returns null; op not called")
        {
            auto empty_array = Value::create(Array{});
            auto reducer = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{reducer});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(empty_array);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            FORBID_CALL(*reducer, call(_));
            FORBID_CALL(*init_expr, evaluate(_));

            ast::Reduce node{std::move(structure_expr),
                             std::move(operation_expr), std::nullopt};

            auto res = node.evaluate(syms);
            CHECK(res->is<Null>());
            CHECK(calls.empty());
        }

        // Frost: reduce [] with fn (acc, elem) -> { acc + elem } init: 42
        SECTION(
            "Empty array with init returns init; init evaluated; op not called")
        {
            auto empty_array = Value::create(Array{});
            auto init_val = Value::create(42_f);
            auto reducer = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{reducer});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(empty_array);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            FORBID_CALL(*reducer, call(_));
            REQUIRE_CALL(*init_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(init_val);

            ast::Reduce node{
                std::move(structure_expr), std::move(operation_expr),
                std::optional<ast::Expression::Ptr>{std::move(init_expr)}};

            auto res = node.evaluate(syms);
            CHECK(res == init_val);
            CHECK(calls.empty());
        }

        // Frost: reduce [] with fn (acc, elem) -> { acc + elem } init: null
        SECTION("Empty array with explicit null init returns null")
        {
            auto empty_array = Value::create(Array{});
            auto init_val = Value::null();
            auto reducer = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{reducer});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(empty_array);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            FORBID_CALL(*reducer, call(_));
            REQUIRE_CALL(*init_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(init_val);

            ast::Reduce node{
                std::move(structure_expr), std::move(operation_expr),
                std::optional<ast::Expression::Ptr>{std::move(init_expr)}};

            auto res = node.evaluate(syms);
            CHECK(res == init_val);
            CHECK(res->is<Null>());
            CHECK(calls.empty());
        }

        // Frost: reduce [1] with fn (acc, elem) -> { acc + elem }
        SECTION("Single element without init returns element; op not called")
        {
            auto elem = Value::create(1_f);
            auto array_val = Value::create(Array{elem});
            auto reducer = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{reducer});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(array_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            FORBID_CALL(*reducer, call(_));
            FORBID_CALL(*init_expr, evaluate(_));

            ast::Reduce node{std::move(structure_expr),
                             std::move(operation_expr), std::nullopt};

            auto res = node.evaluate(syms);
            CHECK(res == elem);
            CHECK(calls.empty());
        }

        // Frost: reduce [1] with fn (acc, elem) -> { acc + elem } init: 10
        SECTION("Single element with init calls op once with (init, elem)")
        {
            auto elem = Value::create(1_f);
            auto array_val = Value::create(Array{elem});
            auto init_val = Value::create(10_f);

            auto reducer = mock::Mock_Callable::make();
            auto result_val = Value::create(99_f);
            auto op_val = Value::create(Function{reducer});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(array_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            REQUIRE_CALL(*init_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(init_val);
            REQUIRE_CALL(*reducer, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .RETURN(result_val);

            ast::Reduce node{
                std::move(structure_expr), std::move(operation_expr),
                std::optional<ast::Expression::Ptr>{std::move(init_expr)}};

            auto res = node.evaluate(syms);
            CHECK(res == result_val);
            REQUIRE(calls.size() == 1);
            REQUIRE(calls.at(0).size() == 2);
            CHECK(calls.at(0).at(0) == init_val);
            CHECK(calls.at(0).at(1) == elem);
        }

        // Frost: reduce [1, 2, 3] with fn (acc, elem) -> { acc + elem }
        SECTION(
            "Multiple elements without init folds left using first two elems")
        {
            auto v1 = Value::create(1_f);
            auto v2 = Value::create(2_f);
            auto v3 = Value::create(3_f);
            auto array_val = Value::create(Array{v1, v2, v3});

            auto reducer = mock::Mock_Callable::make();
            auto r12 = Value::create(12_f);
            auto r123 = Value::create(123_f);
            auto op_val = Value::create(Function{reducer});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(array_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            FORBID_CALL(*init_expr, evaluate(_));
            REQUIRE_CALL(*reducer, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .LR_WITH(_1.size() == 2 && _1[0] == v1 && _1[1] == v2)
                .IN_SEQUENCE(seq)
                .RETURN(r12);
            REQUIRE_CALL(*reducer, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .LR_WITH(_1.size() == 2 && _1[0] == r12 && _1[1] == v3)
                .IN_SEQUENCE(seq)
                .RETURN(r123);

            ast::Reduce node{std::move(structure_expr),
                             std::move(operation_expr), std::nullopt};

            auto res = node.evaluate(syms);
            CHECK(res == r123);
            REQUIRE(calls.size() == 2);
            REQUIRE(calls.at(0).size() == 2);
            CHECK(calls.at(0).at(0) == v1);
            CHECK(calls.at(0).at(1) == v2);
            REQUIRE(calls.at(1).size() == 2);
            CHECK(calls.at(1).at(0) == r12);
            CHECK(calls.at(1).at(1) == v3);
        }

        // Frost: reduce [1, 2, 3] with fn (acc, elem) -> { acc + elem } init:
        // 10
        SECTION("Multiple elements with init folds left starting from init")
        {
            auto v1 = Value::create(1_f);
            auto v2 = Value::create(2_f);
            auto v3 = Value::create(3_f);
            auto array_val = Value::create(Array{v1, v2, v3});
            auto init_val = Value::create(10_f);

            auto reducer = mock::Mock_Callable::make();
            auto r1 = Value::create(11_f);
            auto r2 = Value::create(12_f);
            auto r3 = Value::create(13_f);
            auto op_val = Value::create(Function{reducer});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(array_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            REQUIRE_CALL(*init_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(init_val);
            REQUIRE_CALL(*reducer, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .LR_WITH(_1.size() == 2 && _1[0] == init_val && _1[1] == v1)
                .IN_SEQUENCE(seq)
                .RETURN(r1);
            REQUIRE_CALL(*reducer, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .LR_WITH(_1.size() == 2 && _1[0] == r1 && _1[1] == v2)
                .IN_SEQUENCE(seq)
                .RETURN(r2);
            REQUIRE_CALL(*reducer, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .LR_WITH(_1.size() == 2 && _1[0] == r2 && _1[1] == v3)
                .IN_SEQUENCE(seq)
                .RETURN(r3);

            ast::Reduce node{
                std::move(structure_expr), std::move(operation_expr),
                std::optional<ast::Expression::Ptr>{std::move(init_expr)}};

            auto res = node.evaluate(syms);
            CHECK(res == r3);
            REQUIRE(calls.size() == 3);
            REQUIRE(calls.at(0).size() == 2);
            CHECK(calls.at(0).at(0) == init_val);
            CHECK(calls.at(0).at(1) == v1);
            REQUIRE(calls.at(1).size() == 2);
            CHECK(calls.at(1).at(0) == r1);
            CHECK(calls.at(1).at(1) == v2);
            REQUIRE(calls.at(2).size() == 2);
            CHECK(calls.at(2).at(0) == r2);
            CHECK(calls.at(2).at(1) == v3);
        }

        // Frost: reduce arr_expr with fn (acc, elem) -> { acc + elem } init:
        // init_expr
        SECTION("Evaluation order: structure then operation then init")
        {
            auto elem = Value::create(1_f);
            auto array_val = Value::create(Array{elem});
            auto init_val = Value::create(5_f);

            auto reducer = mock::Mock_Callable::make();
            auto result_val = Value::create(6_f);
            auto op_val = Value::create(Function{reducer});

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(array_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            REQUIRE_CALL(*init_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(init_val);
            REQUIRE_CALL(*reducer, call(_)).RETURN(result_val);

            ast::Reduce node{
                std::move(structure_expr), std::move(operation_expr),
                std::optional<ast::Expression::Ptr>{std::move(init_expr)}};

            auto res = node.evaluate(syms);
            CHECK(res == result_val);
        }

        // Frost: reduce [] with fn (acc, elem) -> { acc + elem } init:
        // init_expr
        SECTION("Init evaluated even when array is empty")
        {
            auto empty_array = Value::create(Array{});
            auto init_val = Value::create(99_f);

            auto reducer = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{reducer});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(empty_array);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            FORBID_CALL(*reducer, call(_));
            REQUIRE_CALL(*init_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(init_val);

            ast::Reduce node{
                std::move(structure_expr), std::move(operation_expr),
                std::optional<ast::Expression::Ptr>{std::move(init_expr)}};

            auto res = node.evaluate(syms);
            CHECK(res == init_val);
            CHECK(calls.empty());
        }
    }

    SECTION("Error cases")
    {
        // Frost: reduce 123 with fn (acc, elem) -> { acc + elem }
        SECTION("Non-structured value errors")
        {
            auto bad_val = Value::create(123_f);

            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .RETURN(bad_val);
            FORBID_CALL(*operation_expr, evaluate(_));
            FORBID_CALL(*init_expr, evaluate(_));

            ast::Reduce node{std::move(structure_expr),
                             std::move(operation_expr), std::nullopt};

            CHECK_THROWS_WITH(
                node.evaluate(syms),
                ContainsSubstring("Cannot reduce value with type"));
        }

        // Frost: reduce [1, 2] with 123
        SECTION("Non-function operation errors")
        {
            auto array_val =
                Value::create(Array{Value::create(1_f), Value::create(2_f)});
            auto bad_op = Value::create(123_f);

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(array_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(bad_op);
            FORBID_CALL(*init_expr, evaluate(_));

            ast::Reduce node{std::move(structure_expr),
                             std::move(operation_expr), std::nullopt};

            CHECK_THROWS_WITH(node.evaluate(syms),
                              "Reduce operation expected Function, got Int");
        }

        // Frost reduce [1, 2] with fn_that_goes_kaboom
        SECTION("Reduction error propagate to caller")
        {
            auto array_val =
                Value::create(Array{Value::create(1_f), Value::create(2_f)});
            auto reducer = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{reducer});

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(array_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            FORBID_CALL(*init_expr, evaluate(_));
            REQUIRE_CALL(*reducer, call(_)).THROW(Frost_Recoverable_Error{"kaboom"});

            ast::Reduce node{std::move(structure_expr),
                             std::move(operation_expr), std::nullopt};

            CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("kaboom"));
        }

        // Frost: reduce arr_expr with fn (acc, elem) -> { acc + elem }
        SECTION("Structure expression error propagates")
        {
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .THROW(Frost_Recoverable_Error{"structure boom"});
            FORBID_CALL(*operation_expr, evaluate(_));
            FORBID_CALL(*init_expr, evaluate(_));

            ast::Reduce node{std::move(structure_expr),
                             std::move(operation_expr), std::nullopt};

            CHECK_THROWS_WITH(node.evaluate(syms),
                              ContainsSubstring("structure boom"));
        }

        // Frost: reduce arr_expr with op_expr init: init_expr
        SECTION("Operation expression error propagates")
        {
            auto array_val =
                Value::create(Array{Value::create(1_f), Value::create(2_f)});

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(array_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .THROW(Frost_Recoverable_Error{"operation boom"});
            FORBID_CALL(*init_expr, evaluate(_));

            ast::Reduce node{std::move(structure_expr),
                             std::move(operation_expr), std::nullopt};

            CHECK_THROWS_WITH(node.evaluate(syms),
                              ContainsSubstring("operation boom"));
        }

        // Frost: reduce arr_expr with fn (acc, elem) -> { acc + elem } init:
        // init_expr
        SECTION("Init expression error propagates and op not called")
        {
            auto array_val = Value::create(Array{Value::create(1_f)});
            auto reducer = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{reducer});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(array_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            FORBID_CALL(*reducer, call(_));
            REQUIRE_CALL(*init_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .THROW(Frost_Recoverable_Error{"init boom"});

            ast::Reduce node{
                std::move(structure_expr), std::move(operation_expr),
                std::optional<ast::Expression::Ptr>{std::move(init_expr)}};

            CHECK_THROWS_WITH(node.evaluate(syms),
                              ContainsSubstring("init boom"));
            CHECK(calls.empty());
        }
    }
}

TEST_CASE("Reduce Map")
{
    // AI-generated test skeleton by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    mock::Mock_Symbol_Table syms;
    auto structure_expr = mock::Mock_Expression::make();
    auto operation_expr = mock::Mock_Expression::make();
    auto init_expr = mock::Mock_Expression::make();

    SECTION("Success cases")
    {
        // Frost: reduce {} with fn (acc, k, v) -> { acc } init: 42
        SECTION(
            "Empty map with init returns init; init evaluated; op not called")
        {
            auto empty_map = Value::create(Map{});
            auto init_val = Value::create(42_f);
            auto reducer = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{reducer});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(empty_map);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            FORBID_CALL(*reducer, call(_));
            REQUIRE_CALL(*init_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(init_val);

            ast::Reduce node{
                std::move(structure_expr), std::move(operation_expr),
                std::optional<ast::Expression::Ptr>{std::move(init_expr)}};

            auto res = node.evaluate(syms);
            CHECK(res == init_val);
            CHECK(calls.empty());
        }

        // Frost: reduce { [1]: "a" } with fn (acc, k, v) -> { acc } init: 0
        SECTION("Single element map calls reducer once")
        {
            auto k1 = Value::create(1_f);
            auto v1 = Value::create("a"s);
            Map map;
            map.insert_or_assign(k1, v1);
            auto map_val = Value::create(std::move(map));
            auto init_val = Value::create(0_f);

            auto reducer = mock::Mock_Callable::make();
            auto r1 = Value::create(7_f);
            auto op_val = Value::create(Function{reducer});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(map_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            REQUIRE_CALL(*init_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(init_val);
            REQUIRE_CALL(*reducer, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .RETURN(r1);

            ast::Reduce node{
                std::move(structure_expr), std::move(operation_expr),
                std::optional<ast::Expression::Ptr>{std::move(init_expr)}};

            auto res = node.evaluate(syms);
            CHECK(res == r1);
            REQUIRE(calls.size() == 1);
            CHECK(calls.at(0).at(0) == init_val);
            CHECK(calls.at(0).at(1) == k1);
            CHECK(calls.at(0).at(2) == v1);
        }

        // Frost: reduce { [1]: "a", [2]: "b" } with fn (acc, k, v) -> { acc }
        // init: 0
        SECTION("Reducer called once per entry with (acc, k, v) in any order")
        {
            auto k1 = Value::create(1_f);
            auto k2 = Value::create(2_f);
            auto v1 = Value::create("a"s);
            auto v2 = Value::create("b"s);

            Map map;
            map.insert_or_assign(k1, v1);
            map.insert_or_assign(k2, v2);
            auto map_val = Value::create(std::move(map));

            auto init_val = Value::create(0_f);
            auto reducer = mock::Mock_Callable::make();
            auto r1 = Value::create(10_f);
            auto r2 = Value::create(20_f);
            auto op_val = Value::create(Function{reducer});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(map_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            REQUIRE_CALL(*init_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(init_val);
            REQUIRE_CALL(*reducer, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .LR_WITH(_1.size() == 3 && _1[0] == init_val)
                .IN_SEQUENCE(seq)
                .RETURN(r1);
            REQUIRE_CALL(*reducer, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .LR_WITH(_1.size() == 3 && _1[0] == r1)
                .IN_SEQUENCE(seq)
                .RETURN(r2);

            ast::Reduce node{
                std::move(structure_expr), std::move(operation_expr),
                std::optional<ast::Expression::Ptr>{std::move(init_expr)}};

            auto res = node.evaluate(syms);
            CHECK(res == r2);
            REQUIRE(calls.size() == 2);

            auto call0 = calls.at(0);
            auto call1 = calls.at(1);
            REQUIRE(call0.size() == 3);
            REQUIRE(call1.size() == 3);

            CHECK(call0.at(0) == init_val);
            CHECK(call1.at(0) == r1);

            auto matches_kv = [&](std::span<const Value_Ptr> call,
                                  const Value_Ptr& k, const Value_Ptr& v) {
                return call.at(1) == k && call.at(2) == v;
            };

            const bool first_is_k1 =
                matches_kv(call0, k1, v1) || matches_kv(call0, k2, v2);
            const bool second_is_k_other =
                matches_kv(call1, k1, v1) || matches_kv(call1, k2, v2);

            CHECK(first_is_k1);
            CHECK(second_is_k_other);

            CHECK(
                ((matches_kv(call0, k1, v1) && matches_kv(call1, k2, v2))
                 || (matches_kv(call0, k2, v2) && matches_kv(call1, k1, v1))));
        }

        // Frost: reduce { [1]: "a" } with fn (acc, k, v) -> { acc } init: null
        SECTION("Null init is allowed")
        {
            auto k1 = Value::create(1_f);
            auto v1 = Value::create("a"s);
            Map map;
            map.insert_or_assign(k1, v1);
            auto map_val = Value::create(std::move(map));
            auto init_val = Value::null();

            auto reducer = mock::Mock_Callable::make();
            auto r1 = Value::create(7_f);
            auto op_val = Value::create(Function{reducer});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(map_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            REQUIRE_CALL(*init_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(init_val);
            REQUIRE_CALL(*reducer, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .RETURN(r1);

            ast::Reduce node{
                std::move(structure_expr), std::move(operation_expr),
                std::optional<ast::Expression::Ptr>{std::move(init_expr)}};

            auto res = node.evaluate(syms);
            CHECK(res == r1);
            REQUIRE(calls.size() == 1);
            CHECK(calls.at(0).at(0) == init_val);
            CHECK(calls.at(0).at(1) == k1);
            CHECK(calls.at(0).at(2) == v1);
        }

        // Frost: reduce m with fn (acc, k, v) -> { acc } init: init_expr
        SECTION("Evaluation order: structure then operation then init")
        {
            auto k1 = Value::create(1_f);
            auto v1 = Value::create("a"s);
            Map map;
            map.insert_or_assign(k1, v1);
            auto map_val = Value::create(std::move(map));
            auto init_val = Value::create(0_f);

            auto reducer = mock::Mock_Callable::make();
            auto r1 = Value::create(7_f);
            auto op_val = Value::create(Function{reducer});

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(map_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            REQUIRE_CALL(*init_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(init_val);
            REQUIRE_CALL(*reducer, call(_)).RETURN(r1);

            ast::Reduce node{
                std::move(structure_expr), std::move(operation_expr),
                std::optional<ast::Expression::Ptr>{std::move(init_expr)}};

            auto res = node.evaluate(syms);
            CHECK(res == r1);
        }
    }

    SECTION("Error cases")
    {
        // Frost: reduce {} with fn (acc, k, v) -> { acc }
        SECTION("Missing init errors (operation may be evaluated)")
        {
            auto empty_map = Value::create(Map{});
            auto reducer = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{reducer});

            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .RETURN(empty_map);
            ALLOW_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .RETURN(op_val);
            FORBID_CALL(*init_expr, evaluate(_));
            FORBID_CALL(*reducer, call(_));

            ast::Reduce node{std::move(structure_expr),
                             std::move(operation_expr), std::nullopt};

            CHECK_THROWS_WITH(node.evaluate(syms),
                              ContainsSubstring("Map reduction requires init"));
        }

        // Frost: reduce 123 with fn (acc, k, v) -> { acc } init: 0
        SECTION("Non-structured value errors and op not evaluated")
        {
            auto bad_val = Value::create(123_f);

            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .RETURN(bad_val);
            FORBID_CALL(*operation_expr, evaluate(_));
            FORBID_CALL(*init_expr, evaluate(_));

            ast::Reduce node{std::move(structure_expr),
                             std::move(operation_expr), std::nullopt};

            CHECK_THROWS_WITH(
                node.evaluate(syms),
                ContainsSubstring("Cannot reduce value with type"));
        }

        // Frost: reduce { [1]: "a" } with 123 init: 0
        SECTION("Non-function operation errors")
        {
            auto k1 = Value::create(1_f);
            auto v1 = Value::create("a"s);
            Map map;
            map.insert_or_assign(k1, v1);
            auto map_val = Value::create(std::move(map));
            auto bad_op = Value::create(123_f);

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(map_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(bad_op);
            FORBID_CALL(*init_expr, evaluate(_));

            ast::Reduce node{
                std::move(structure_expr), std::move(operation_expr),
                std::optional<ast::Expression::Ptr>{std::move(init_expr)}};

            CHECK_THROWS_WITH(node.evaluate(syms),
                              "Reduce operation expected Function, got Int");
        }

        // Frost: reduce { [1]: "a" } with fn_that_goes_kaboom init: 0
        SECTION("Reducer error propagates")
        {
            auto k1 = Value::create(1_f);
            auto v1 = Value::create("a"s);
            Map map;
            map.insert_or_assign(k1, v1);
            auto map_val = Value::create(std::move(map));
            auto reducer = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{reducer});
            auto init_val = Value::create(0_f);

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(map_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            REQUIRE_CALL(*init_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(init_val);
            REQUIRE_CALL(*reducer, call(_)).THROW(Frost_Recoverable_Error{"kaboom"});

            ast::Reduce node{
                std::move(structure_expr), std::move(operation_expr),
                std::optional<ast::Expression::Ptr>{std::move(init_expr)}};

            CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("kaboom"));
        }

        // Frost: reduce m with fn (acc, k, v) -> { acc } init: init_expr
        SECTION("Init expression error propagates")
        {
            auto k1 = Value::create(1_f);
            auto v1 = Value::create("a"s);
            Map map;
            map.insert_or_assign(k1, v1);
            auto map_val = Value::create(std::move(map));
            auto reducer = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{reducer});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(map_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            FORBID_CALL(*reducer, call(_));
            REQUIRE_CALL(*init_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .THROW(Frost_Recoverable_Error{"init boom"});

            ast::Reduce node{
                std::move(structure_expr), std::move(operation_expr),
                std::optional<ast::Expression::Ptr>{std::move(init_expr)}};

            CHECK_THROWS_WITH(node.evaluate(syms),
                              ContainsSubstring("init boom"));
            CHECK(calls.empty());
        }

        // Frost: reduce m with op_expr init: 0
        SECTION("Operation expression error propagates")
        {
            auto k1 = Value::create(1_f);
            auto v1 = Value::create("a"s);
            Map map;
            map.insert_or_assign(k1, v1);
            auto map_val = Value::create(std::move(map));

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(map_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .THROW(Frost_Recoverable_Error{"operation boom"});
            FORBID_CALL(*init_expr, evaluate(_));

            ast::Reduce node{
                std::move(structure_expr), std::move(operation_expr),
                std::optional<ast::Expression::Ptr>{std::move(init_expr)}};

            CHECK_THROWS_WITH(node.evaluate(syms),
                              ContainsSubstring("operation boom"));
        }

        // Frost: reduce m with fn (acc, k, v) -> { acc } init: 0
        SECTION("Structure expression error propagates")
        {
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .THROW(Frost_Recoverable_Error{"structure boom"});
            FORBID_CALL(*operation_expr, evaluate(_));
            FORBID_CALL(*init_expr, evaluate(_));

            ast::Reduce node{std::move(structure_expr),
                             std::move(operation_expr), std::nullopt};

            CHECK_THROWS_WITH(node.evaluate(syms),
                              ContainsSubstring("structure boom"));
        }
    }
}
