#include <catch2/catch_test_macros.hpp>
#include <catch2/trompeloeil.hpp>

#include <frost/mock/mock-symbol-table.hpp>
#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/ast/match-binding.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <ranges>
#include <vector>

using namespace frst;
using namespace frst::ast;
using namespace std::literals;

using trompeloeil::_;

namespace
{

// Minimal Callable for constructing Function-typed values.
struct Dummy_Callable : Callable
{
    Value_Ptr call(std::span<const Value_Ptr>) const override
    {
        return Value::null();
    }
    std::string debug_dump() const override
    {
        return "<dummy>";
    }
    std::string name() const override
    {
        return "<dummy>";
    }
};

Value_Ptr make_function()
{
    return Value::create(Function{std::make_shared<Dummy_Callable>()});
}

Match_Binding::Ptr make_binding(std::optional<std::string> name,
                                std::optional<Type_Constraint> constraint)
{
    return std::make_unique<Match_Binding>(AST_Node::no_range, std::move(name),
                                           constraint);
}

} // namespace

TEST_CASE("Match_Binding: try_match without constraint")
{
    SECTION("Named binding binds value and returns true")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto val = Value::create(42_f);
        REQUIRE_CALL(syms, define("x", val));

        auto binding = make_binding("x"s, std::nullopt);
        CHECK(binding->try_match(ctx, val));
    }

    SECTION("Discard binding returns true without binding")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        FORBID_CALL(syms, define(_, _));

        auto binding = make_binding(std::nullopt, std::nullopt);
        CHECK(binding->try_match(ctx, Value::create(42_f)));
    }

    SECTION("Unconstrained binding accepts any value type")
    {
        std::vector<Value_Ptr> values = {
            Value::create(42_f),  Value::create(3.14),
            Value::create(true),  Value::create("hello"s),
            Value::null(),        Value::create(Array{}),
            Value::create(Map{}), make_function(),
        };

        for (const auto& v : values)
        {
            mock::Mock_Symbol_Table syms;
            Execution_Context ctx{.symbols = syms};
            REQUIRE_CALL(syms, define("v", v));
            auto binding = make_binding("v"s, std::nullopt);
            CHECK(binding->try_match(ctx, v));
        }
    }
}

TEST_CASE("Match_Binding: try_match with constraint")
{
    SECTION("Bind path: matching constraint binds and returns true")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto val = Value::create(42_f);
        REQUIRE_CALL(syms, define("n", val));

        auto binding = make_binding("n"s, Type_Constraint::Int);
        CHECK(binding->try_match(ctx, val));
    }

    SECTION("Bind path: mismatched constraint returns false without binding")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        FORBID_CALL(syms, define(_, _));

        auto binding = make_binding("n"s, Type_Constraint::Int);
        CHECK_FALSE(binding->try_match(ctx, Value::create("not int"s)));
    }

    SECTION("Discard path: matching constraint succeeds without binding")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        FORBID_CALL(syms, define(_, _));

        auto binding = make_binding(std::nullopt, Type_Constraint::Float);
        CHECK(binding->try_match(ctx, Value::create(3.14)));
    }

    SECTION("Discard path: mismatched constraint returns false")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        FORBID_CALL(syms, define(_, _));

        auto binding = make_binding(std::nullopt, Type_Constraint::Bool);
        CHECK_FALSE(binding->try_match(ctx, Value::create(42_f)));
    }
}

TEST_CASE("Match_Binding: each concrete type constraint")
{
    struct Case
    {
        Type_Constraint constraint;
        Value_Ptr positive; // value that should match
        Value_Ptr negative; // value that should not match
    };

    const std::vector<Case> cases = [] {
        std::vector<Case> v;
        v.push_back({Type_Constraint::Null, Value::null(), Value::create(0_f)});
        v.push_back(
            {Type_Constraint::Int, Value::create(42_f), Value::create(3.14)});
        v.push_back(
            {Type_Constraint::Float, Value::create(3.14), Value::create(42_f)});
        v.push_back(
            {Type_Constraint::Bool, Value::create(true), Value::create(42_f)});
        v.push_back({Type_Constraint::String, Value::create("s"s),
                     Value::create(42_f)});
        v.push_back({Type_Constraint::Array, Value::create(Array{}),
                     Value::create(42_f)});
        v.push_back(
            {Type_Constraint::Map, Value::create(Map{}), Value::create(42_f)});
        v.push_back(
            {Type_Constraint::Function, make_function(), Value::create(42_f)});
        return v;
    }();

    for (const auto& c : cases)
    {
        DYNAMIC_SECTION(TC::to_string(c.constraint) << " matches positive")
        {
            mock::Mock_Symbol_Table syms;
            Execution_Context ctx{.symbols = syms};
            REQUIRE_CALL(syms, define("x", c.positive));
            auto binding = make_binding("x"s, c.constraint);
            CHECK(binding->try_match(ctx, c.positive));
        }

        DYNAMIC_SECTION(TC::to_string(c.constraint) << " rejects negative")
        {
            mock::Mock_Symbol_Table syms;
            Execution_Context ctx{.symbols = syms};
            FORBID_CALL(syms, define(_, _));
            auto binding = make_binding("x"s, c.constraint);
            CHECK_FALSE(binding->try_match(ctx, c.negative));
        }
    }
}

TEST_CASE("Match_Binding: category constraints")
{
    struct Typed_Value
    {
        std::string name;
        Value_Ptr value;
    };
    const std::vector<Typed_Value> all_types = {
        {"Null", Value::null()},         {"Bool", Value::create(true)},
        {"Int", Value::create(42_f)},    {"Float", Value::create(3.14)},
        {"String", Value::create("s"s)}, {"Array", Value::create(Array{})},
        {"Map", Value::create(Map{})},   {"Function", make_function()},
    };

    // Uses a real Symbol_Table (not a mock) so we can verify both the
    // return value and the binding state with a single `has(...)` check,
    // without having to thread conditional expectations through the mock.
    auto check_match = [](Type_Constraint c, const Value_Ptr& v,
                          bool expected) {
        Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};
        auto binding = make_binding("x"s, c);
        CHECK(binding->try_match(ctx, v) == expected);
        CHECK(syms.has("x") == expected);
    };

    SECTION("Numeric matches Int and Float only")
    {
        for (const auto& [name, v] : all_types)
        {
            DYNAMIC_SECTION(name)
            {
                const bool expected = (name == "Int" || name == "Float");
                check_match(Type_Constraint::Numeric, v, expected);
            }
        }
    }

    SECTION("Primitive matches Null, Bool, Int, Float, String")
    {
        for (const auto& [name, v] : all_types)
        {
            DYNAMIC_SECTION(name)
            {
                const bool expected = (name
                                       == "Null"
                                       || name
                                       == "Bool"
                                       || name
                                       == "Int"
                                       || name
                                       == "Float"
                                       || name
                                       == "String");
                check_match(Type_Constraint::Primitive, v, expected);
            }
        }
    }

    SECTION("Structured matches Array and Map only")
    {
        for (const auto& [name, v] : all_types)
        {
            DYNAMIC_SECTION(name)
            {
                const bool expected = (name == "Array" || name == "Map");
                check_match(Type_Constraint::Structured, v, expected);
            }
        }
    }

    SECTION("Nonnull matches everything except Null")
    {
        for (const auto& [name, v] : all_types)
        {
            DYNAMIC_SECTION(name)
            {
                const bool expected = (name != "Null");
                check_match(Type_Constraint::Nonnull, v, expected);
            }
        }
    }
}

TEST_CASE("Match_Binding: duplicate binding is unrecoverable")
{
    // Binding a name that is already defined in the target symbol table
    // propagates Frost_Unrecoverable_Error from Symbol_Table::define -- this
    // is how Match_Binding signals "this isn't a match failure, it's a bug
    // in the pattern or enclosing scope."
    Symbol_Table syms;
    syms.define("x", Value::create(1_f));
    Execution_Context ctx{.symbols = syms};

    auto binding = make_binding("x"s, std::nullopt);
    CHECK_THROWS_AS(binding->try_match(ctx, Value::create(2_f)),
                    Frost_Unrecoverable_Error);
}

TEST_CASE("Match_Binding: constraint check runs before bind attempt")
{
    // A pattern with a constraint that fails against the value should
    // NEVER reach the define() call -- even if the name would collide with
    // an existing binding. This confirms the check-then-bind ordering: a
    // failed match is cleanly non-fatal, only a successful match can
    // trigger the unrecoverable duplicate-binding path.
    Symbol_Table syms;
    syms.define("x", Value::create(1_f));
    Execution_Context ctx{.symbols = syms};

    // "x is String" applied to an Int -- constraint fails first.
    auto binding = make_binding("x"s, Type_Constraint::String);
    CHECK_FALSE(binding->try_match(ctx, Value::create(42_f)));
    // Reached here without throwing -> define() was never called.
}

TEST_CASE("Match_Binding: symbol_sequence")
{
    SECTION("Named binding yields one Definition")
    {
        auto binding = make_binding("foo"s, std::nullopt);
        auto actions =
            binding->symbol_sequence() | std::ranges::to<std::vector>();
        REQUIRE(actions.size() == 1);
        auto* def = std::get_if<AST_Node::Definition>(&actions[0]);
        REQUIRE(def);
        CHECK(def->name == "foo");
        CHECK(def->exported == false);
    }

    SECTION("Discard binding yields nothing")
    {
        auto binding = make_binding(std::nullopt, std::nullopt);
        auto actions =
            binding->symbol_sequence() | std::ranges::to<std::vector>();
        CHECK(actions.empty());
    }

    SECTION("Constraint does not affect symbol_sequence (named)")
    {
        auto binding = make_binding("foo"s, Type_Constraint::Int);
        auto actions =
            binding->symbol_sequence() | std::ranges::to<std::vector>();
        REQUIRE(actions.size() == 1);
        auto* def = std::get_if<AST_Node::Definition>(&actions[0]);
        REQUIRE(def);
        CHECK(def->name == "foo");
    }

    SECTION("Constraint does not affect symbol_sequence (discard)")
    {
        auto binding = make_binding(std::nullopt, Type_Constraint::Int);
        auto actions =
            binding->symbol_sequence() | std::ranges::to<std::vector>();
        CHECK(actions.empty());
    }
}

TEST_CASE("Match_Binding: node_label")
{
    SECTION("Named, no constraint")
    {
        auto binding = make_binding("foo"s, std::nullopt);
        CHECK(binding->node_label() == "Match_Binding(foo)");
    }

    SECTION("Discard, no constraint")
    {
        auto binding = make_binding(std::nullopt, std::nullopt);
        CHECK(binding->node_label() == "Match_Binding(_)");
    }

    SECTION("Named, with constraint")
    {
        auto binding = make_binding("foo"s, Type_Constraint::Int);
        CHECK(binding->node_label() == "Match_Binding(foo is Int)");
    }

    SECTION("Discard, with constraint")
    {
        auto binding = make_binding(std::nullopt, Type_Constraint::Float);
        CHECK(binding->node_label() == "Match_Binding(_ is Float)");
    }

    SECTION("Category constraints render by name")
    {
        auto a = make_binding("x"s, Type_Constraint::Numeric);
        CHECK(a->node_label() == "Match_Binding(x is Numeric)");

        auto b = make_binding("x"s, Type_Constraint::Primitive);
        CHECK(b->node_label() == "Match_Binding(x is Primitive)");

        auto c = make_binding("x"s, Type_Constraint::Structured);
        CHECK(c->node_label() == "Match_Binding(x is Structured)");
    }
}

TEST_CASE("Match_Binding: Type_Constraint helpers")
{
    SECTION("to_string maps each value to its enum name")
    {
        CHECK(TC::to_string(Type_Constraint::Null) == "Null");
        CHECK(TC::to_string(Type_Constraint::Int) == "Int");
        CHECK(TC::to_string(Type_Constraint::Float) == "Float");
        CHECK(TC::to_string(Type_Constraint::Bool) == "Bool");
        CHECK(TC::to_string(Type_Constraint::String) == "String");
        CHECK(TC::to_string(Type_Constraint::Array) == "Array");
        CHECK(TC::to_string(Type_Constraint::Map) == "Map");
        CHECK(TC::to_string(Type_Constraint::Function) == "Function");
        CHECK(TC::to_string(Type_Constraint::Primitive) == "Primitive");
        CHECK(TC::to_string(Type_Constraint::Numeric) == "Numeric");
        CHECK(TC::to_string(Type_Constraint::Structured) == "Structured");
        CHECK(TC::to_string(Type_Constraint::Nonnull) == "Nonnull");
    }

    SECTION("from_string maps each enum name to its value")
    {
        CHECK(TC::from_string("Null") == Type_Constraint::Null);
        CHECK(TC::from_string("Int") == Type_Constraint::Int);
        CHECK(TC::from_string("Float") == Type_Constraint::Float);
        CHECK(TC::from_string("Bool") == Type_Constraint::Bool);
        CHECK(TC::from_string("String") == Type_Constraint::String);
        CHECK(TC::from_string("Array") == Type_Constraint::Array);
        CHECK(TC::from_string("Map") == Type_Constraint::Map);
        CHECK(TC::from_string("Function") == Type_Constraint::Function);
        CHECK(TC::from_string("Primitive") == Type_Constraint::Primitive);
        CHECK(TC::from_string("Numeric") == Type_Constraint::Numeric);
        CHECK(TC::from_string("Structured") == Type_Constraint::Structured);
        CHECK(TC::from_string("Nonnull") == Type_Constraint::Nonnull);
    }

    SECTION("from_string returns nullopt for unrecognized strings")
    {
        CHECK(TC::from_string("") == std::nullopt);
        CHECK(TC::from_string("int") == std::nullopt); // case-sensitive
        CHECK(TC::from_string("integer") == std::nullopt);
        CHECK(TC::from_string("Unknown") == std::nullopt);
        // Sentinel names are not user-facing:
        CHECK(TC::from_string("Unconstrained") == std::nullopt);
        CHECK(TC::from_string("None") == std::nullopt);
    }

    SECTION("Round-trip: from_string(to_string(x)) == x for all values")
    {
        const std::vector<Type_Constraint> all_values{
            Type_Constraint::Null,       Type_Constraint::Int,
            Type_Constraint::Float,      Type_Constraint::Bool,
            Type_Constraint::String,     Type_Constraint::Array,
            Type_Constraint::Map,        Type_Constraint::Function,
            Type_Constraint::Primitive,  Type_Constraint::Numeric,
            Type_Constraint::Structured, Type_Constraint::Nonnull,
        };
        for (auto v : all_values)
            CHECK(TC::from_string(TC::to_string(v)) == v);
    }
}
