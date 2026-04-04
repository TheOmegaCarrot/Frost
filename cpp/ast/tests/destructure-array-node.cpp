#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <catch2/trompeloeil.hpp>

#include <frost/mock/mock-destructure.hpp>
#include <frost/mock/mock-symbol-table.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/ast/destructure-array.hpp>
#include <frost/value.hpp>

#include <ranges>
#include <vector>

using namespace frst;
using namespace frst::ast;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;

using trompeloeil::_;

TEST_CASE("Destructure_Array")
{
    SECTION("Delegates to children in order")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto mock_a = mock::Mock_Destructure::make();
        auto mock_b = mock::Mock_Destructure::make();
        auto* a = mock_a.get();
        auto* b = mock_b.get();

        auto val_a = Value::create(1_f);
        auto val_b = Value::create(2_f);
        auto arr = Value::create(Array{val_a, val_b});

        std::vector<Destructure::Ptr> children;
        children.push_back(std::move(mock_a));
        children.push_back(std::move(mock_b));

        trompeloeil::sequence seq;
        REQUIRE_CALL(*a, do_destructure(_, val_a)).IN_SEQUENCE(seq);
        REQUIRE_CALL(*b, do_destructure(_, val_b)).IN_SEQUENCE(seq);

        Destructure_Array node{AST_Node::no_range, std::move(children),
                               std::nullopt, false};
        node.destructure(ctx, arr);
    }

    SECTION("Too few elements throws")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto mock_a = mock::Mock_Destructure::make();
        auto mock_b = mock::Mock_Destructure::make();

        std::vector<Destructure::Ptr> children;
        children.push_back(std::move(mock_a));
        children.push_back(std::move(mock_b));

        auto arr = Value::create(Array{Value::create(1_f)});

        Destructure_Array node{AST_Node::no_range, std::move(children),
                               std::nullopt, false};

        CHECK_THROWS_WITH(node.destructure(ctx, arr),
                          ContainsSubstring("Insufficient"));
    }

    SECTION("Too many elements without rest throws")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto mock_a = mock::Mock_Destructure::make();
        auto* a = mock_a.get();

        std::vector<Destructure::Ptr> children;
        children.push_back(std::move(mock_a));

        auto arr = Value::create(
            Array{Value::create(1_f), Value::create(2_f)});

        Destructure_Array node{AST_Node::no_range, std::move(children),
                               std::nullopt, false};

        // The first child would be called before the size check fails,
        // so allow it
        ALLOW_CALL(*a, do_destructure(_, _));

        CHECK_THROWS_WITH(node.destructure(ctx, arr),
                          ContainsSubstring("Too many"));
    }

    SECTION("Rest binding collects remaining elements")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto mock_a = mock::Mock_Destructure::make();
        auto* a = mock_a.get();

        auto val_a = Value::create(1_f);
        auto val_b = Value::create(2_f);
        auto val_c = Value::create(3_f);
        auto arr = Value::create(Array{val_a, val_b, val_c});

        std::vector<Destructure::Ptr> children;
        children.push_back(std::move(mock_a));

        REQUIRE_CALL(*a, do_destructure(_, val_a));
        Value_Ptr captured_rest;
        REQUIRE_CALL(syms, define("rest", _))
            .LR_SIDE_EFFECT(captured_rest = _2);

        Destructure_Array node{AST_Node::no_range, std::move(children),
                               "rest", false};
        node.destructure(ctx, arr);

        REQUIRE(captured_rest->is<Array>());
        const auto& rest_arr = captured_rest->raw_get<Array>();
        REQUIRE(rest_arr.size() == 2);
        CHECK(rest_arr[0] == val_b);
        CHECK(rest_arr[1] == val_c);
    }

    SECTION("Rest binding with exact match produces empty array")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto mock_a = mock::Mock_Destructure::make();
        auto* a = mock_a.get();

        auto val = Value::create(1_f);
        auto arr = Value::create(Array{val});

        std::vector<Destructure::Ptr> children;
        children.push_back(std::move(mock_a));

        Value_Ptr captured_rest;
        REQUIRE_CALL(*a, do_destructure(_, val));
        REQUIRE_CALL(syms, define("rest", _))
            .LR_SIDE_EFFECT(captured_rest = _2);

        Destructure_Array node{AST_Node::no_range, std::move(children),
                               "rest", false};
        node.destructure(ctx, arr);

        REQUIRE(captured_rest->is<Array>());
        CHECK(captured_rest->raw_get<Array>().empty());
    }

    SECTION("Rest discard ..._ does not define")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto mock_a = mock::Mock_Destructure::make();
        auto* a = mock_a.get();

        auto val = Value::create(1_f);
        auto arr = Value::create(
            Array{val, Value::create(2_f), Value::create(3_f)});

        std::vector<Destructure::Ptr> children;
        children.push_back(std::move(mock_a));

        REQUIRE_CALL(*a, do_destructure(_, val));
        FORBID_CALL(syms, define(_, _));

        Destructure_Array node{AST_Node::no_range, std::move(children),
                               "_", false};
        node.destructure(ctx, arr);
    }

    SECTION("Non-Array value throws")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        std::vector<Destructure::Ptr> children;

        Destructure_Array node{AST_Node::no_range, std::move(children),
                               std::nullopt, false};

        CHECK_THROWS_WITH(node.destructure(ctx, Value::create(42_f)),
                          ContainsSubstring("Array"));
        CHECK_THROWS_WITH(node.destructure(ctx, Value::create("hi"s)),
                          ContainsSubstring("String"));
        CHECK_THROWS_WITH(node.destructure(ctx, Value::create(Map{})),
                          ContainsSubstring("Map"));
    }

    SECTION("Empty array with zero children succeeds")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        std::vector<Destructure::Ptr> children;
        auto arr = Value::create(Array{});

        Destructure_Array node{AST_Node::no_range, std::move(children),
                               std::nullopt, false};
        node.destructure(ctx, arr);
    }

    SECTION("Children receive pointer-identical values")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto mock_a = mock::Mock_Destructure::make();
        auto* a = mock_a.get();

        auto val = Value::create(42_f);
        auto arr = Value::create(Array{val});

        std::vector<Destructure::Ptr> children;
        children.push_back(std::move(mock_a));

        // Check pointer identity, not just value equality
        REQUIRE_CALL(*a, do_destructure(_, _))
            .WITH(_2.get() == val.get());

        Destructure_Array node{AST_Node::no_range, std::move(children),
                               std::nullopt, false};
        node.destructure(ctx, arr);
    }

    SECTION("symbol_sequence includes rest Definition")
    {
        std::vector<Destructure::Ptr> children;

        Destructure_Array node{AST_Node::no_range, std::move(children),
                               "rest", true};

        auto actions =
            node.symbol_sequence() | std::ranges::to<std::vector>();
        REQUIRE(actions.size() == 1);
        auto* def = std::get_if<AST_Node::Definition>(&actions[0]);
        REQUIRE(def);
        CHECK(def->name == "rest");
        CHECK(def->exported == true);
    }

    SECTION("symbol_sequence: rest _ yields no Definition")
    {
        std::vector<Destructure::Ptr> children;

        Destructure_Array node{AST_Node::no_range, std::move(children),
                               "_", false};

        auto actions =
            node.symbol_sequence() | std::ranges::to<std::vector>();
        CHECK(actions.empty());
    }

    SECTION("node_label without rest")
    {
        std::vector<Destructure::Ptr> children;
        Destructure_Array node{AST_Node::no_range, std::move(children),
                               std::nullopt, false};
        CHECK(node.node_label() == "Destructure_Array");
    }

    SECTION("node_label with rest")
    {
        std::vector<Destructure::Ptr> children;
        Destructure_Array node{AST_Node::no_range, std::move(children),
                               "tail", false};
        CHECK(node.node_label() == "Destructure_Array(rest: tail)");
    }

    SECTION("Child error propagates and stops iteration")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto mock_a = mock::Mock_Destructure::make();
        auto mock_b = mock::Mock_Destructure::make();
        auto* a = mock_a.get();
        auto* b = mock_b.get();

        auto arr = Value::create(
            Array{Value::create(1_f), Value::create(2_f)});

        std::vector<Destructure::Ptr> children;
        children.push_back(std::move(mock_a));
        children.push_back(std::move(mock_b));

        REQUIRE_CALL(*a, do_destructure(_, _))
            .THROW(Frost_Recoverable_Error{"child boom"});
        FORBID_CALL(*b, do_destructure(_, _));

        Destructure_Array node{AST_Node::no_range, std::move(children),
                               std::nullopt, false};

        CHECK_THROWS_WITH(node.destructure(ctx, arr),
                          ContainsSubstring("child boom"));
    }

    SECTION("Non-Array values: Null, Bool, Function")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        std::vector<Destructure::Ptr> children;
        Destructure_Array node{AST_Node::no_range, std::move(children),
                               std::nullopt, false};

        CHECK_THROWS_WITH(node.destructure(ctx, Value::null()),
                          ContainsSubstring("Null"));
        CHECK_THROWS_WITH(node.destructure(ctx, Value::create(true)),
                          ContainsSubstring("Bool"));
    }
}
