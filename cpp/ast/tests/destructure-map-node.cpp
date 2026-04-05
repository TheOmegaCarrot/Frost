#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <catch2/trompeloeil.hpp>

#include <frost/mock/mock-destructure.hpp>
#include <frost/mock/mock-expression.hpp>
#include <frost/mock/mock-symbol-table.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/ast/destructure-map.hpp>
#include <frost/ast/name-lookup.hpp>
#include <frost/value.hpp>

#include <ranges>
#include <vector>

using namespace frst;
using namespace std::literals;
using frst::ast::AST_Node;
using frst::ast::Destructure;
using frst::ast::Destructure_Map;
using Catch::Matchers::ContainsSubstring;

using trompeloeil::_;

TEST_CASE("Destructure_Map")
{
    SECTION("Delegates to children with matching values")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto key_a_expr = mock::Mock_Expression::make();
        auto key_b_expr = mock::Mock_Expression::make();
        auto* key_a_ptr = key_a_expr.get();
        auto* key_b_ptr = key_b_expr.get();

        auto mock_a = mock::Mock_Destructure::make();
        auto mock_b = mock::Mock_Destructure::make();
        auto* a = mock_a.get();
        auto* b = mock_b.get();

        auto key_a = Value::create("a"s);
        auto key_b = Value::create("b"s);
        auto val_a = Value::create(1_f);
        auto val_b = Value::create(2_f);

        auto map = Value::create(Map{{key_a, val_a}, {key_b, val_b}});

        REQUIRE_CALL(*key_a_ptr, do_evaluate(_)).RETURN(key_a);
        REQUIRE_CALL(*key_b_ptr, do_evaluate(_)).RETURN(key_b);
        REQUIRE_CALL(*a, do_destructure(_, val_a));
        REQUIRE_CALL(*b, do_destructure(_, val_b));

        std::vector<Destructure_Map::Element> elems;
        elems.push_back({std::move(key_a_expr), std::move(mock_a)});
        elems.push_back({std::move(key_b_expr), std::move(mock_b)});

        Destructure_Map node{AST_Node::no_range, std::move(elems)};
        node.destructure(ctx, map);
    }

    SECTION("Missing key passes null to child")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto key_expr = mock::Mock_Expression::make();
        auto* key_ptr = key_expr.get();

        auto mock_child = mock::Mock_Destructure::make();
        auto* child = mock_child.get();

        auto key = Value::create("missing"s);
        auto map = Value::create(Map{});

        REQUIRE_CALL(*key_ptr, do_evaluate(_)).RETURN(key);
        REQUIRE_CALL(*child, do_destructure(_, Value::null()));

        std::vector<Destructure_Map::Element> elems;
        elems.push_back({std::move(key_expr), std::move(mock_child)});

        Destructure_Map node{AST_Node::no_range, std::move(elems)};
        node.destructure(ctx, map);
    }

    SECTION("Non-string primitive keys work")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto key_expr = mock::Mock_Expression::make();
        auto* key_ptr = key_expr.get();

        auto mock_child = mock::Mock_Destructure::make();
        auto* child = mock_child.get();

        auto key = Value::create(42_f);
        auto val = Value::create("found"s);
        auto map = Value::create(Map{{key, val}});

        REQUIRE_CALL(*key_ptr, do_evaluate(_)).RETURN(key);
        REQUIRE_CALL(*child, do_destructure(_, val));

        std::vector<Destructure_Map::Element> elems;
        elems.push_back({std::move(key_expr), std::move(mock_child)});

        Destructure_Map node{AST_Node::no_range, std::move(elems)};
        node.destructure(ctx, map);
    }

    SECTION("Null key throws")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto key_expr = mock::Mock_Expression::make();
        auto* key_ptr = key_expr.get();

        auto mock_child = mock::Mock_Destructure::make();

        REQUIRE_CALL(*key_ptr, do_evaluate(_)).RETURN(Value::null());

        std::vector<Destructure_Map::Element> elems;
        elems.push_back({std::move(key_expr), std::move(mock_child)});

        Destructure_Map node{AST_Node::no_range, std::move(elems)};

        CHECK_THROWS_WITH(node.destructure(ctx, Value::create(Map{})),
                          ContainsSubstring("valid Map keys"));
    }

    SECTION("Non-primitive key throws")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto key_expr = mock::Mock_Expression::make();
        auto* key_ptr = key_expr.get();

        auto mock_child = mock::Mock_Destructure::make();

        REQUIRE_CALL(*key_ptr, do_evaluate(_))
            .RETURN(Value::create(Array{}));

        std::vector<Destructure_Map::Element> elems;
        elems.push_back({std::move(key_expr), std::move(mock_child)});

        Destructure_Map node{AST_Node::no_range, std::move(elems)};

        CHECK_THROWS_WITH(node.destructure(ctx, Value::create(Map{})),
                          ContainsSubstring("valid Map keys"));
    }

    SECTION("Non-Map value throws")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        std::vector<Destructure_Map::Element> elems;
        Destructure_Map node{AST_Node::no_range, std::move(elems)};

        CHECK_THROWS_WITH(node.destructure(ctx, Value::create(42_f)),
                          ContainsSubstring("Map"));
        CHECK_THROWS_WITH(node.destructure(ctx, Value::create(Array{})),
                          ContainsSubstring("Map"));
        CHECK_THROWS_WITH(node.destructure(ctx, Value::create("hi"s)),
                          ContainsSubstring("Map"));
    }

    SECTION("Empty map with zero elements succeeds")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        std::vector<Destructure_Map::Element> elems;
        Destructure_Map node{AST_Node::no_range, std::move(elems)};

        node.destructure(ctx, Value::create(Map{}));
    }

    SECTION("Extra map keys are ignored")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto key_expr = mock::Mock_Expression::make();
        auto* key_ptr = key_expr.get();

        auto mock_child = mock::Mock_Destructure::make();
        auto* child = mock_child.get();

        auto key_a = Value::create("a"s);
        auto val_a = Value::create(1_f);
        auto key_b = Value::create("b"s);
        auto val_b = Value::create(2_f);
        auto map = Value::create(Map{{key_a, val_a}, {key_b, val_b}});

        // Only destructure "a", ignore "b"
        REQUIRE_CALL(*key_ptr, do_evaluate(_)).RETURN(key_a);
        REQUIRE_CALL(*child, do_destructure(_, val_a));

        std::vector<Destructure_Map::Element> elems;
        elems.push_back({std::move(key_expr), std::move(mock_child)});

        Destructure_Map node{AST_Node::no_range, std::move(elems)};
        node.destructure(ctx, map);
    }

    SECTION("Children receive pointer-identical values")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto key_expr = mock::Mock_Expression::make();
        auto* key_ptr = key_expr.get();

        auto mock_child = mock::Mock_Destructure::make();
        auto* child = mock_child.get();

        auto key = Value::create("k"s);
        auto val = Value::create(42_f);
        auto map = Value::create(Map{{key, val}});

        REQUIRE_CALL(*key_ptr, do_evaluate(_)).RETURN(key);
        REQUIRE_CALL(*child, do_destructure(_, _))
            .WITH(_2.get() == val.get());

        std::vector<Destructure_Map::Element> elems;
        elems.push_back({std::move(key_expr), std::move(mock_child)});

        Destructure_Map node{AST_Node::no_range, std::move(elems)};
        node.destructure(ctx, map);
    }

    SECTION("symbol_sequence delegates to children")
    {
        auto key_expr = mock::Mock_Expression::make();
        auto mock_child = mock::Mock_Destructure::make();

        // Mock_Expression and Mock_Destructure have empty symbol_sequences
        std::vector<Destructure_Map::Element> elems;
        elems.push_back({std::move(key_expr), std::move(mock_child)});

        Destructure_Map node{AST_Node::no_range, std::move(elems)};

        auto actions =
            node.symbol_sequence() | std::ranges::to<std::vector>();
        CHECK(actions.empty());
    }

    SECTION("symbol_sequence includes key expression usages")
    {
        // Use a Name_Lookup as the key expression so it yields a Usage
        auto key_expr = std::make_unique<frst::ast::Name_Lookup>(
            AST_Node::no_range, "my_key");
        auto mock_child = mock::Mock_Destructure::make();

        std::vector<Destructure_Map::Element> elems;
        elems.push_back({std::move(key_expr), std::move(mock_child)});

        Destructure_Map node{AST_Node::no_range, std::move(elems)};

        auto actions =
            node.symbol_sequence() | std::ranges::to<std::vector>();
        REQUIRE(actions.size() == 1);
        auto* usage = std::get_if<AST_Node::Usage>(&actions[0]);
        REQUIRE(usage);
        CHECK(usage->name == "my_key");
    }

    SECTION("node_label")
    {
        std::vector<Destructure_Map::Element> elems;
        Destructure_Map node{AST_Node::no_range, std::move(elems)};
        CHECK(node.node_label() == "Destructure_Map");
    }

    SECTION("Child error propagates")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto key_expr = mock::Mock_Expression::make();
        auto* key_ptr = key_expr.get();

        auto mock_child = mock::Mock_Destructure::make();
        auto* child = mock_child.get();

        auto key = Value::create("a"s);
        auto val = Value::create(1_f);
        auto map = Value::create(Map{{key, val}});

        REQUIRE_CALL(*key_ptr, do_evaluate(_)).RETURN(key);
        REQUIRE_CALL(*child, do_destructure(_, _))
            .THROW(Frost_Recoverable_Error{"child boom"});

        std::vector<Destructure_Map::Element> elems;
        elems.push_back({std::move(key_expr), std::move(mock_child)});

        Destructure_Map node{AST_Node::no_range, std::move(elems)};

        CHECK_THROWS_WITH(node.destructure(ctx, map),
                          ContainsSubstring("child boom"));
    }

    SECTION("Mixed found and missing keys")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto key_found_expr = mock::Mock_Expression::make();
        auto key_missing_expr = mock::Mock_Expression::make();
        auto* key_found_ptr = key_found_expr.get();
        auto* key_missing_ptr = key_missing_expr.get();

        auto mock_found = mock::Mock_Destructure::make();
        auto mock_missing = mock::Mock_Destructure::make();
        auto* found = mock_found.get();
        auto* missing = mock_missing.get();

        auto key_a = Value::create("a"s);
        auto key_b = Value::create("b"s);
        auto val_a = Value::create(1_f);
        auto map = Value::create(Map{{key_a, val_a}});

        REQUIRE_CALL(*key_found_ptr, do_evaluate(_)).RETURN(key_a);
        REQUIRE_CALL(*key_missing_ptr, do_evaluate(_)).RETURN(key_b);
        REQUIRE_CALL(*found, do_destructure(_, val_a));
        REQUIRE_CALL(*missing, do_destructure(_, Value::null()));

        std::vector<Destructure_Map::Element> elems;
        elems.push_back({std::move(key_found_expr), std::move(mock_found)});
        elems.push_back(
            {std::move(key_missing_expr), std::move(mock_missing)});

        Destructure_Map node{AST_Node::no_range, std::move(elems)};
        node.destructure(ctx, map);
    }

    SECTION("Bool key works")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto key_expr = mock::Mock_Expression::make();
        auto* key_ptr = key_expr.get();

        auto mock_child = mock::Mock_Destructure::make();
        auto* child = mock_child.get();

        auto key = Value::create(true);
        auto val = Value::create("yes"s);
        auto map = Value::create(Map{{key, val}});

        REQUIRE_CALL(*key_ptr, do_evaluate(_)).RETURN(key);
        REQUIRE_CALL(*child, do_destructure(_, val));

        std::vector<Destructure_Map::Element> elems;
        elems.push_back({std::move(key_expr), std::move(mock_child)});

        Destructure_Map node{AST_Node::no_range, std::move(elems)};
        node.destructure(ctx, map);
    }

    SECTION("Key expression error propagates")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto key_expr = mock::Mock_Expression::make();
        auto* key_ptr = key_expr.get();

        auto mock_child = mock::Mock_Destructure::make();

        REQUIRE_CALL(*key_ptr, do_evaluate(_))
            .THROW(Frost_Recoverable_Error{"key boom"});
        FORBID_CALL(*mock_child.get(), do_destructure(_, _));

        std::vector<Destructure_Map::Element> elems;
        elems.push_back({std::move(key_expr), std::move(mock_child)});

        Destructure_Map node{AST_Node::no_range, std::move(elems)};

        CHECK_THROWS_WITH(node.destructure(ctx, Value::create(Map{})),
                          ContainsSubstring("key boom"));
    }

    SECTION("Duplicate key expressions both receive the value")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto key_expr_1 = mock::Mock_Expression::make();
        auto key_expr_2 = mock::Mock_Expression::make();
        auto* key_ptr_1 = key_expr_1.get();
        auto* key_ptr_2 = key_expr_2.get();

        auto mock_a = mock::Mock_Destructure::make();
        auto mock_b = mock::Mock_Destructure::make();
        auto* a = mock_a.get();
        auto* b = mock_b.get();

        auto key = Value::create("x"s);
        auto val = Value::create(42_f);
        auto map = Value::create(Map{{key, val}});

        REQUIRE_CALL(*key_ptr_1, do_evaluate(_)).RETURN(key);
        REQUIRE_CALL(*key_ptr_2, do_evaluate(_)).RETURN(key);
        REQUIRE_CALL(*a, do_destructure(_, val));
        REQUIRE_CALL(*b, do_destructure(_, val));

        std::vector<Destructure_Map::Element> elems;
        elems.push_back({std::move(key_expr_1), std::move(mock_a)});
        elems.push_back({std::move(key_expr_2), std::move(mock_b)});

        Destructure_Map node{AST_Node::no_range, std::move(elems)};
        node.destructure(ctx, map);
    }
}
