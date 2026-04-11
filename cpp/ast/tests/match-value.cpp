#include <catch2/catch_test_macros.hpp>
#include <catch2/trompeloeil.hpp>

#include <frost/mock/mock-expression.hpp>
#include <frost/mock/mock-symbol-table.hpp>
#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/ast/match-value.hpp>
#include <frost/ast/name-lookup.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <memory>
#include <ranges>
#include <vector>

using namespace frst;
using namespace frst::ast;
using namespace std::literals;

using trompeloeil::_;

namespace
{

Match_Value::Ptr make_value_pattern(Expression::Ptr expr)
{
    return std::make_unique<Match_Value>(AST_Node::no_range, std::move(expr));
}

} // namespace

TEST_CASE("Match_Value: evaluates expression and compares to scrutinee")
{
    mock::Mock_Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};

    auto expr = mock::Mock_Expression::make();
    auto expected_value = Value::create(42_f);

    REQUIRE_CALL(*expr, do_evaluate(_))
        .LR_WITH(&_1.symbols == &syms)
        .RETURN(expected_value);

    auto pat = make_value_pattern(std::move(expr));

    SECTION("Equal values produce a successful match")
    {
        CHECK(pat->try_match(ctx, expected_value));
    }
}

TEST_CASE("Match_Value: mismatch returns false")
{
    mock::Mock_Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};

    auto expr = mock::Mock_Expression::make();
    REQUIRE_CALL(*expr, do_evaluate(_)).RETURN(Value::create(42_f));

    auto pat = make_value_pattern(std::move(expr));
    CHECK_FALSE(pat->try_match(ctx, Value::create(41_f)));
}

TEST_CASE("Match_Value: equality across primitive types")
{
    // Each row: the value the expression yields, and a scrutinee. `match`
    // is the expected truth value of equality under Frost's `==`.
    struct Case
    {
        std::string description;
        Value_Ptr expr_value;
        Value_Ptr scrutinee;
        bool match;
    };

    const std::vector<Case> cases = {
        {"Int == same Int", Value::create(42_f), Value::create(42_f), true},
        {"Int != different Int", Value::create(42_f), Value::create(41_f),
         false},
        {"Float == same Float", Value::create(3.14), Value::create(3.14),
         true},
        {"Float != different Float", Value::create(3.14), Value::create(2.71),
         false},
        {"Bool == same Bool", Value::create(true), Value::create(true), true},
        {"Bool != opposite Bool", Value::create(true), Value::create(false),
         false},
        {"String == same String", Value::create("hello"s),
         Value::create("hello"s), true},
        {"String != different String", Value::create("hello"s),
         Value::create("world"s), false},
        {"Null == Null", Value::null(), Value::null(), true},
        {"Null != Int 0", Value::null(), Value::create(0_f), false},
        {"Null != Bool false", Value::null(), Value::create(false), false},

        // Cross-type comparisons: Frost `==` is type-strict.
        {"Int 3 != Float 3.0", Value::create(3_f), Value::create(3.0), false},
        {"Float 3.0 != Int 3", Value::create(3.0), Value::create(3_f), false},
        {"Int 42 != String \"42\"", Value::create(42_f),
         Value::create("42"s), false},
        {"Bool true != Int 1", Value::create(true), Value::create(1_f),
         false},
        {"Bool false != Int 0", Value::create(false), Value::create(0_f),
         false},
    };

    for (const auto& c : cases)
    {
        DYNAMIC_SECTION(c.description)
        {
            mock::Mock_Symbol_Table syms;
            Execution_Context ctx{.symbols = syms};

            auto expr = mock::Mock_Expression::make();
            REQUIRE_CALL(*expr, do_evaluate(_)).RETURN(c.expr_value);

            auto pat = make_value_pattern(std::move(expr));
            CHECK(pat->try_match(ctx, c.scrutinee) == c.match);
        }
    }
}

TEST_CASE("Match_Value: equality of structured values")
{
    auto run = [](Value_Ptr expr_value, Value_Ptr scrutinee, bool match) {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto expr = mock::Mock_Expression::make();
        REQUIRE_CALL(*expr, do_evaluate(_)).RETURN(expr_value);

        auto pat = make_value_pattern(std::move(expr));
        CHECK(pat->try_match(ctx, scrutinee) == match);
    };

    SECTION("Equal arrays match")
    {
        run(Value::create(Array{Value::create(1_f), Value::create(2_f)}),
            Value::create(Array{Value::create(1_f), Value::create(2_f)}),
            true);
    }

    SECTION("Arrays differing in length do not match")
    {
        run(Value::create(Array{Value::create(1_f), Value::create(2_f)}),
            Value::create(Array{Value::create(1_f)}), false);
    }

    SECTION("Arrays differing in content do not match")
    {
        run(Value::create(Array{Value::create(1_f), Value::create(2_f)}),
            Value::create(Array{Value::create(1_f), Value::create(99_f)}),
            false);
    }

    SECTION("Empty arrays match")
    {
        run(Value::create(Array{}), Value::create(Array{}), true);
    }

    SECTION("Equal maps match")
    {
        // Fully qualify frst::Map to disambiguate from frst::ast::Map (the
        // AST node for map literal expressions), which is in scope through
        // the umbrella ast.hpp pulled in transitively.
        run(Value::create(frst::Map{
                {Value::create("a"s), Value::create(1_f)},
                {Value::create("b"s), Value::create(2_f)},
            }),
            Value::create(frst::Map{
                {Value::create("a"s), Value::create(1_f)},
                {Value::create("b"s), Value::create(2_f)},
            }),
            true);
    }

    SECTION("Maps differing in a value do not match")
    {
        run(Value::create(
                frst::Map{{Value::create("a"s), Value::create(1_f)}}),
            Value::create(
                frst::Map{{Value::create("a"s), Value::create(99_f)}}),
            false);
    }

    SECTION("Empty maps match")
    {
        run(Value::create(frst::Map{}), Value::create(frst::Map{}), true);
    }
}

TEST_CASE("Match_Value: never defines any name")
{
    // Whether the match succeeds or fails, Match_Value must not introduce
    // any binding into the symbol table.
    mock::Mock_Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};

    FORBID_CALL(syms, define(_, _));

    SECTION("Successful match does not define")
    {
        auto expr = mock::Mock_Expression::make();
        REQUIRE_CALL(*expr, do_evaluate(_)).RETURN(Value::create(42_f));

        auto pat = make_value_pattern(std::move(expr));
        CHECK(pat->try_match(ctx, Value::create(42_f)));
    }

    SECTION("Failed match does not define")
    {
        auto expr = mock::Mock_Expression::make();
        REQUIRE_CALL(*expr, do_evaluate(_)).RETURN(Value::create(42_f));

        auto pat = make_value_pattern(std::move(expr));
        CHECK_FALSE(pat->try_match(ctx, Value::create(99_f)));
    }
}

TEST_CASE("Match_Value: errors from the expression propagate")
{
    // A runtime error during pattern expression evaluation should NOT be
    // silently swallowed as a match failure -- it propagates out of
    // try_match, matching the behavior of guards and if/elif chains.
    mock::Mock_Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};

    auto expr = mock::Mock_Expression::make();
    REQUIRE_CALL(*expr, do_evaluate(_))
        .THROW(Frost_Recoverable_Error{"kaboom"});

    auto pat = make_value_pattern(std::move(expr));
    CHECK_THROWS_AS(pat->try_match(ctx, Value::create(42_f)),
                    Frost_Recoverable_Error);
}

TEST_CASE("Match_Value: children exposes the expression")
{
    auto inner = mock::Mock_Expression::make();
    auto* inner_raw = inner.get();

    auto pat = make_value_pattern(std::move(inner));
    auto kids = pat->children() | std::ranges::to<std::vector>();
    REQUIRE(kids.size() == 1);
    CHECK(kids[0].node == inner_raw);
}

TEST_CASE("Match_Value: node_label")
{
    auto pat = make_value_pattern(mock::Mock_Expression::make());
    CHECK(pat->node_label() == "Match_Value");
}

TEST_CASE("Match_Value: symbol_sequence propagates the expression's usages")
{
    // Match_Value doesn't override symbol_sequence, so the base-class
    // implementation walks children. A Name_Lookup child yields a Usage
    // action, which Match_Value's symbol_sequence passes through.
    auto pat = make_value_pattern(
        std::make_unique<Name_Lookup>(AST_Node::no_range, "foo"s));

    auto actions = pat->symbol_sequence() | std::ranges::to<std::vector>();
    REQUIRE(actions.size() == 1);
    auto* usage = std::get_if<AST_Node::Usage>(&actions[0]);
    REQUIRE(usage);
    CHECK(usage->name == "foo");
}
