#include <catch2/catch_test_macros.hpp>

#include <catch2/trompeloeil.hpp>

#include <frost/mock/mock-symbol-table.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/ast/destructure-leaf.hpp>
#include <frost/value.hpp>

#include <ranges>
#include <vector>

using namespace frst;
using namespace frst::ast;
using namespace std::literals;

using trompeloeil::_;

TEST_CASE("Destructure_Leaf")
{
    SECTION("Binds name to value")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto val = Value::create(42_f);
        Destructure_Leaf leaf{AST_Node::no_range, "x", false};

        REQUIRE_CALL(syms, define("x", val));

        leaf.destructure(ctx, val);
    }

    SECTION("Discard _ does not define")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        FORBID_CALL(syms, define(_, _));

        Destructure_Leaf leaf{AST_Node::no_range, std::nullopt, false};
        leaf.destructure(ctx, Value::create(42_f));
    }

    SECTION("Works with various value types")
    {
        std::vector<Value_Ptr> values = {
            Value::create(42_f),  Value::create("hello"s),
            Value::create(3.14),  Value::create(true),
            Value::null(),        Value::create(Array{Value::create(1_f)}),
            Value::create(Map{}),
        };

        for (const auto& val : values)
        {
            mock::Mock_Symbol_Table syms;
            Execution_Context ctx{.symbols = syms};

            REQUIRE_CALL(syms, define("v", val));

            Destructure_Leaf leaf{AST_Node::no_range, "v", false};
            leaf.destructure(ctx, val);
        }
    }

    SECTION("Export flag does not affect destructure behavior")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto val = Value::create(42_f);

        REQUIRE_CALL(syms, define("x", val));

        Destructure_Leaf leaf{AST_Node::no_range, "x", true};
        leaf.destructure(ctx, val);
    }

    SECTION("symbol_sequence: normal name, not exported")
    {
        Destructure_Leaf leaf{AST_Node::no_range, "foo", false};

        auto actions = leaf.symbol_sequence() | std::ranges::to<std::vector>();
        REQUIRE(actions.size() == 1);
        auto* def = std::get_if<AST_Node::Definition>(&actions[0]);
        REQUIRE(def);
        CHECK(def->name == "foo");
        CHECK(def->exported == false);
    }

    SECTION("symbol_sequence: normal name, exported")
    {
        Destructure_Leaf leaf{AST_Node::no_range, "bar", true};

        auto actions = leaf.symbol_sequence() | std::ranges::to<std::vector>();
        REQUIRE(actions.size() == 1);
        auto* def = std::get_if<AST_Node::Definition>(&actions[0]);
        REQUIRE(def);
        CHECK(def->name == "bar");
        CHECK(def->exported == true);
    }

    SECTION("symbol_sequence: discard _ yields nothing")
    {
        Destructure_Leaf leaf{AST_Node::no_range, std::nullopt, false};

        auto actions = leaf.symbol_sequence() | std::ranges::to<std::vector>();
        CHECK(actions.empty());
    }

    SECTION("Empty name is rejected")
    {
        CHECK_THROWS_AS(Destructure_Leaf(AST_Node::no_range, "", false),
                        Frost_Interpreter_Error);
    }

    SECTION("Exported discard _ does not define and yields nothing")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        FORBID_CALL(syms, define(_, _));

        Destructure_Leaf leaf{AST_Node::no_range, std::nullopt, true};
        leaf.destructure(ctx, Value::create(42_f));

        auto actions = leaf.symbol_sequence() | std::ranges::to<std::vector>();
        CHECK(actions.empty());
    }

    SECTION("node_label")
    {
        Destructure_Leaf leaf{AST_Node::no_range, "myvar", false};
        CHECK(leaf.node_label() == "Destructure_Leaf(myvar)");

        Destructure_Leaf discard{AST_Node::no_range, std::nullopt, false};
        CHECK(discard.node_label() == "Destructure_Leaf(discarded)");
    }
}
