#include <catch2/catch_test_macros.hpp>

#include <catch2/trompeloeil.hpp>

#include <frost/mock/mock-symbol-table.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/ast/destructure-binding.hpp>
#include <frost/value.hpp>

#include <ranges>
#include <vector>

using namespace frst;
using namespace frst::ast;
using namespace std::literals;

using trompeloeil::_;

TEST_CASE("Destructure_Binding")
{
    SECTION("Binds name to value")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto val = Value::create(42_f);
        Destructure_Binding binding{AST_Node::no_range, "x"};

        REQUIRE_CALL(syms, define("x", val));

        binding.destructure(ctx, val);
    }

    SECTION("Discard _ does not define")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        FORBID_CALL(syms, define(_, _));

        Destructure_Binding binding{AST_Node::no_range, std::nullopt};
        binding.destructure(ctx, Value::create(42_f));
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

            Destructure_Binding binding{AST_Node::no_range, "v"};
            binding.destructure(ctx, val);
        }
    }

    SECTION("symbol_sequence: normal name")
    {
        Destructure_Binding binding{AST_Node::no_range, "foo"};

        auto actions =
            binding.symbol_sequence() | std::ranges::to<std::vector>();
        REQUIRE(actions.size() == 1);
        auto* def = std::get_if<AST_Node::Definition>(&actions[0]);
        REQUIRE(def);
        CHECK(def->name == "foo");
    }

    SECTION("symbol_sequence: discard _ yields nothing")
    {
        Destructure_Binding binding{AST_Node::no_range, std::nullopt};

        auto actions =
            binding.symbol_sequence() | std::ranges::to<std::vector>();
        CHECK(actions.empty());
    }

    SECTION("Empty name is rejected")
    {
        CHECK_THROWS_AS(Destructure_Binding(AST_Node::no_range, ""),
                        Frost_Interpreter_Error);
    }

    SECTION("node_label")
    {
        Destructure_Binding binding{AST_Node::no_range, "myvar"};
        CHECK(binding.node_label() == "Destructure_Binding(myvar)");

        Destructure_Binding discard{AST_Node::no_range, std::nullopt};
        CHECK(discard.node_label() == "Destructure_Binding(discarded)");
    }
}
