#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>
#include <catch2/trompeloeil.hpp>

#include <frost/mock/mock-expression.hpp>
#include <frost/mock/mock-symbol-table.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/ast.hpp>
#include <frost/symbol-table.hpp>

#include "../reduce.hpp"

using namespace frst;
using namespace std::literals;

using Catch::Matchers::ContainsSubstring;
using trompeloeil::_;

namespace
{
struct Recording_Reducer final : Callable
{
    mutable std::vector<std::vector<Value_Ptr>> calls;
    mutable std::size_t call_index = 0;
    std::vector<Value_Ptr> results;

    Value_Ptr call(const std::vector<Value_Ptr>& args) const override
    {
        calls.push_back(args);
        if (call_index < results.size())
            return results.at(call_index++);
        return Value::create();
    }

    std::string debug_dump() const override
    {
        return "<recording>";
    }
};

struct Throwing_Reducer final : Callable
{
    Value_Ptr call(const std::vector<Value_Ptr>&) const override
    {
        throw Frost_Error{"kaboom"};
    }

    std::string debug_dump() const override
    {
        return "<throwing>";
    }
};
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
            auto reducer = std::make_shared<Recording_Reducer>();
            auto op_val = Value::create(Function{reducer});

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(empty_array);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            FORBID_CALL(*init_expr, evaluate(_));

            ast::Reduce node{std::move(structure_expr),
                             std::move(operation_expr), std::nullopt};

            auto res = node.evaluate(syms);
            CHECK(res->is<Null>());
            CHECK(reducer->calls.empty());
        }

        // Frost: reduce [] with fn (acc, elem) -> { acc + elem } init: 42
        SECTION(
            "Empty array with init returns init; init evaluated; op not called")
        {
            auto empty_array = Value::create(Array{});
            auto init_val = Value::create(42_f);
            auto reducer = std::make_shared<Recording_Reducer>();
            auto op_val = Value::create(Function{reducer});

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(empty_array);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            REQUIRE_CALL(*init_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(init_val);

            ast::Reduce node{
                std::move(structure_expr), std::move(operation_expr),
                std::optional<ast::Expression::Ptr>{std::move(init_expr)}};

            auto res = node.evaluate(syms);
            CHECK(res == init_val);
            CHECK(reducer->calls.empty());
        }

        // Frost: reduce [1] with fn (acc, elem) -> { acc + elem }
        SECTION("Single element without init returns element; op not called")
        {
            auto elem = Value::create(1_f);
            auto array_val = Value::create(Array{elem});
            auto reducer = std::make_shared<Recording_Reducer>();
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

            ast::Reduce node{std::move(structure_expr),
                             std::move(operation_expr), std::nullopt};

            auto res = node.evaluate(syms);
            CHECK(res == elem);
            CHECK(reducer->calls.empty());
        }

        // Frost: reduce [1] with fn (acc, elem) -> { acc + elem } init: 10
        SECTION("Single element with init calls op once with (init, elem)")
        {
            auto elem = Value::create(1_f);
            auto array_val = Value::create(Array{elem});
            auto init_val = Value::create(10_f);

            auto reducer = std::make_shared<Recording_Reducer>();
            auto result_val = Value::create(99_f);
            reducer->results = {result_val};
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

            ast::Reduce node{
                std::move(structure_expr), std::move(operation_expr),
                std::optional<ast::Expression::Ptr>{std::move(init_expr)}};

            auto res = node.evaluate(syms);
            CHECK(res == result_val);
            REQUIRE(reducer->calls.size() == 1);
            REQUIRE(reducer->calls.at(0).size() == 2);
            CHECK(reducer->calls.at(0).at(0) == init_val);
            CHECK(reducer->calls.at(0).at(1) == elem);
        }

        // Frost: reduce [1, 2, 3] with fn (acc, elem) -> { acc + elem }
        SECTION(
            "Multiple elements without init folds left using first two elems")
        {
            auto v1 = Value::create(1_f);
            auto v2 = Value::create(2_f);
            auto v3 = Value::create(3_f);
            auto array_val = Value::create(Array{v1, v2, v3});

            auto reducer = std::make_shared<Recording_Reducer>();
            auto r12 = Value::create(12_f);
            auto r123 = Value::create(123_f);
            reducer->results = {r12, r123};
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

            ast::Reduce node{std::move(structure_expr),
                             std::move(operation_expr), std::nullopt};

            auto res = node.evaluate(syms);
            CHECK(res == r123);
            REQUIRE(reducer->calls.size() == 2);
            REQUIRE(reducer->calls.at(0).size() == 2);
            CHECK(reducer->calls.at(0).at(0) == v1);
            CHECK(reducer->calls.at(0).at(1) == v2);
            REQUIRE(reducer->calls.at(1).size() == 2);
            CHECK(reducer->calls.at(1).at(0) == r12);
            CHECK(reducer->calls.at(1).at(1) == v3);
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

            auto reducer = std::make_shared<Recording_Reducer>();
            auto r1 = Value::create(11_f);
            auto r2 = Value::create(12_f);
            auto r3 = Value::create(13_f);
            reducer->results = {r1, r2, r3};
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

            ast::Reduce node{
                std::move(structure_expr), std::move(operation_expr),
                std::optional<ast::Expression::Ptr>{std::move(init_expr)}};

            auto res = node.evaluate(syms);
            CHECK(res == r3);
            REQUIRE(reducer->calls.size() == 3);
            REQUIRE(reducer->calls.at(0).size() == 2);
            CHECK(reducer->calls.at(0).at(0) == init_val);
            CHECK(reducer->calls.at(0).at(1) == v1);
            REQUIRE(reducer->calls.at(1).size() == 2);
            CHECK(reducer->calls.at(1).at(0) == r1);
            CHECK(reducer->calls.at(1).at(1) == v2);
            REQUIRE(reducer->calls.at(2).size() == 2);
            CHECK(reducer->calls.at(2).at(0) == r2);
            CHECK(reducer->calls.at(2).at(1) == v3);
        }

        // Frost: reduce arr_expr with fn (acc, elem) -> { acc + elem } init:
        // init_expr
        SECTION("Evaluation order: structure then operation then init")
        {
            auto elem = Value::create(1_f);
            auto array_val = Value::create(Array{elem});
            auto init_val = Value::create(5_f);

            auto reducer = std::make_shared<Recording_Reducer>();
            auto result_val = Value::create(6_f);
            reducer->results = {result_val};
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

            auto reducer = std::make_shared<Recording_Reducer>();
            auto op_val = Value::create(Function{reducer});

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(empty_array);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            REQUIRE_CALL(*init_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(init_val);

            ast::Reduce node{
                std::move(structure_expr), std::move(operation_expr),
                std::optional<ast::Expression::Ptr>{std::move(init_expr)}};

            auto res = node.evaluate(syms);
            CHECK(res == init_val);
            CHECK(reducer->calls.empty());
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

            CHECK_THROWS_WITH(
                node.evaluate(syms),
                ContainsSubstring("Reduce with expected Function"));
        }

        // Frost reduce [1, 2] with fn_that_goes_kaboom
        SECTION("Reduction error propagate to caller")
        {
            auto array_val =
                Value::create(Array{Value::create(1_f), Value::create(2_f)});
            auto reducer = std::make_shared<Throwing_Reducer>();
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

            ast::Reduce node{std::move(structure_expr),
                             std::move(operation_expr), std::nullopt};

            CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("kaboom"));
        }

        // Frost: reduce arr_expr with fn (acc, elem) -> { acc + elem }
        SECTION("Structure expression error propagates")
        {
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .THROW(Frost_Error{"structure boom"});
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
                .THROW(Frost_Error{"operation boom"});
            FORBID_CALL(*init_expr, evaluate(_));

            ast::Reduce node{std::move(structure_expr),
                             std::move(operation_expr), std::nullopt};

            CHECK_THROWS_WITH(node.evaluate(syms),
                              ContainsSubstring("operation boom"));
        }

        // Frost: reduce arr_expr with fn (acc, elem) -> { acc + elem } init: init_expr
        SECTION("Init expression error propagates and op not called")
        {
            auto array_val = Value::create(Array{Value::create(1_f)});
            auto reducer = std::make_shared<Recording_Reducer>();
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
                .THROW(Frost_Error{"init boom"});

            ast::Reduce node{
                std::move(structure_expr), std::move(operation_expr),
                std::optional<ast::Expression::Ptr>{std::move(init_expr)}};

            CHECK_THROWS_WITH(node.evaluate(syms),
                              ContainsSubstring("init boom"));
            CHECK(reducer->calls.empty());
        }
    }
}
