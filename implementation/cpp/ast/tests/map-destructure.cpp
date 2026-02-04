#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_exception.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>
#include <catch2/trompeloeil.hpp>

#include <frost/mock/mock-expression.hpp>
#include <frost/mock/mock-symbol-table.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/ast.hpp>
#include <frost/value.hpp>

#include <generator>
#include <ranges>
#include <string>
#include <string_view>
#include <vector>

using namespace frst;
using namespace frst::ast;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;
using Catch::Matchers::Equals;
using Catch::Matchers::MessageMatches;
using trompeloeil::_;

namespace
{
class Sequence_Expression final : public Expression
{
  public:
    Sequence_Expression(std::vector<Statement::Symbol_Action> actions,
                        Value_Ptr result = Value::null())
        : actions_{std::move(actions)}
        , result_{std::move(result)}
    {
    }

    Value_Ptr evaluate(const Symbol_Table&) const override
    {
        return result_;
    }

    std::generator<Statement::Symbol_Action> symbol_sequence() const override
    {
        for (const auto& action : actions_)
            co_yield action;
    }

  protected:
    std::string node_label() const override
    {
        return "Sequence_Expression";
    }

  private:
    std::vector<Statement::Symbol_Action> actions_;
    Value_Ptr result_;
};

std::string action_to_string(const Statement::Symbol_Action& action)
{
    return action.visit(Overload{
        [](const Statement::Definition& action) {
            return "def:" + action.name;
        },
        [](const Statement::Usage& action) {
            return "use:" + action.name;
        },
    });
}

std::vector<std::string> collect_sequence(const Statement& node)
{
    return node.symbol_sequence()
           | std::views::transform(&action_to_string)
           | std::ranges::to<std::vector>();
}
} // namespace

TEST_CASE("Map_Destructure")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).

    SECTION("Defines names and binds null for missing keys")
    {
        auto rhs_expr = mock::Mock_Expression::make();
        auto key_expr = mock::Mock_Expression::make();
        auto missing_key_expr = mock::Mock_Expression::make();
        mock::Mock_Symbol_Table syms;
        trompeloeil::sequence seq;

        auto value = Value::create("beep"s);
        auto rhs_map = Value::create(frst::Map{{Value::create("foo"s), value}});
        auto key_val = Value::create("foo"s);
        auto missing_key_val = Value::create("missing"s);

        REQUIRE_CALL(*rhs_expr, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(rhs_map);
        REQUIRE_CALL(*key_expr, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(key_val);
        REQUIRE_CALL(syms, define("bar", value)).IN_SEQUENCE(seq);
        REQUIRE_CALL(*missing_key_expr, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(missing_key_val);
        REQUIRE_CALL(syms, define("answer", _))
            .IN_SEQUENCE(seq)
            .LR_SIDE_EFFECT({ CHECK(_2->template is<Null>()); });

        std::vector<Map_Destructure::Element> elems;
        elems.emplace_back(
            Map_Destructure::Element{std::move(key_expr), "bar"});
        elems.emplace_back(
            Map_Destructure::Element{std::move(missing_key_expr), "answer"});
        Map_Destructure node{std::move(elems), std::move(rhs_expr)};

        auto result = node.execute(syms);
        CHECK_FALSE(result.has_value());
    }

    SECTION("Exports bound names when enabled")
    {
        auto rhs_expr = mock::Mock_Expression::make();
        auto key_expr = mock::Mock_Expression::make();
        mock::Mock_Symbol_Table syms;
        trompeloeil::sequence seq;

        auto value = Value::create(42_f);
        auto rhs_map = Value::create(frst::Map{{Value::create("foo"s), value}});
        auto key_val = Value::create("foo"s);

        REQUIRE_CALL(*rhs_expr, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(rhs_map);
        REQUIRE_CALL(*key_expr, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(key_val);
        REQUIRE_CALL(syms, define("bar", value)).IN_SEQUENCE(seq);

        std::vector<Map_Destructure::Element> elems;
        elems.emplace_back(
            Map_Destructure::Element{std::move(key_expr), "bar"});
        Map_Destructure node{std::move(elems), std::move(rhs_expr), true};

        auto result = node.execute(syms);
        REQUIRE(result.has_value());
        CHECK(result->size() == 1);

        auto it = result->find(Value::create("bar"s));
        REQUIRE(it != result->end());
        CHECK(it->second == value);
    }

    SECTION("Exports multiple bound names when enabled")
    {
        auto rhs_expr = mock::Mock_Expression::make();
        auto key_expr1 = mock::Mock_Expression::make();
        auto key_expr2 = mock::Mock_Expression::make();
        mock::Mock_Symbol_Table syms;
        trompeloeil::sequence seq;

        auto v1 = Value::create(1_f);
        auto v2 = Value::create(2_f);
        auto rhs_map = Value::create(
            frst::Map{{Value::create("k1"s), v1}, {Value::create("k2"s), v2}});

        REQUIRE_CALL(*rhs_expr, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(rhs_map);
        REQUIRE_CALL(*key_expr1, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(Value::create("k1"s));
        REQUIRE_CALL(syms, define("a", v1)).IN_SEQUENCE(seq);
        REQUIRE_CALL(*key_expr2, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(Value::create("k2"s));
        REQUIRE_CALL(syms, define("b", v2)).IN_SEQUENCE(seq);

        std::vector<Map_Destructure::Element> elems;
        elems.emplace_back(Map_Destructure::Element{std::move(key_expr1), "a"});
        elems.emplace_back(Map_Destructure::Element{std::move(key_expr2), "b"});
        Map_Destructure node{std::move(elems), std::move(rhs_expr), true};

        auto result = node.execute(syms);
        REQUIRE(result.has_value());
        CHECK(result->size() == 2);

        auto it_a = result->find(Value::create("a"s));
        REQUIRE(it_a != result->end());
        CHECK(it_a->second == v1);

        auto it_b = result->find(Value::create("b"s));
        REQUIRE(it_b != result->end());
        CHECK(it_b->second == v2);
    }

    SECTION("Exports null for missing keys when enabled")
    {
        auto rhs_expr = mock::Mock_Expression::make();
        auto key_expr = mock::Mock_Expression::make();
        mock::Mock_Symbol_Table syms;
        trompeloeil::sequence seq;

        auto rhs_map = Value::create(frst::Map{});
        auto key_val = Value::create("missing"s);

        REQUIRE_CALL(*rhs_expr, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(rhs_map);
        REQUIRE_CALL(*key_expr, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(key_val);
        REQUIRE_CALL(syms, define("bar", _))
            .IN_SEQUENCE(seq)
            .LR_WITH(_2->template is<Null>());

        std::vector<Map_Destructure::Element> elems;
        elems.emplace_back(
            Map_Destructure::Element{std::move(key_expr), "bar"});
        Map_Destructure node{std::move(elems), std::move(rhs_expr), true};

        auto result = node.execute(syms);
        REQUIRE(result.has_value());
        CHECK(result->size() == 1);

        auto it = result->find(Value::create("bar"s));
        REQUIRE(it != result->end());
        CHECK(it->second->template is<Null>());
    }

    SECTION("Duplicate binding names are rejected")
    {
        std::vector<Map_Destructure::Element> elems;
        elems.emplace_back(
            Map_Destructure::Element{mock::Mock_Expression::make(), "dup"});
        elems.emplace_back(
            Map_Destructure::Element{mock::Mock_Expression::make(), "dup"});

        CHECK_THROWS_MATCHES(
            (Map_Destructure{std::move(elems), mock::Mock_Expression::make()}),
            Frost_Unrecoverable_Error,
            MessageMatches(
                Equals("Duplicate destructuring binding name: dup")));
    }

    SECTION("Non-map RHS errors and does not evaluate keys")
    {
        auto rhs_expr = mock::Mock_Expression::make();
        auto key_expr = mock::Mock_Expression::make();
        mock::Mock_Symbol_Table syms;

        REQUIRE_CALL(*rhs_expr, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .RETURN(Value::create(1_f));
        FORBID_CALL(*key_expr, evaluate(_));
        FORBID_CALL(syms, define(_, _));

        std::vector<Map_Destructure::Element> elems;
        elems.emplace_back(
            Map_Destructure::Element{std::move(key_expr), "bar"});
        Map_Destructure node{std::move(elems), std::move(rhs_expr)};

        CHECK_THROWS_MATCHES(
            node.execute(syms), Frost_Recoverable_Error,
            MessageMatches(Equals("Cannot destructure Int to Map")));
    }

    SECTION("Non-primitive key expressions error")
    {
        auto rhs_expr = mock::Mock_Expression::make();
        auto key_expr = mock::Mock_Expression::make();
        mock::Mock_Symbol_Table syms;

        REQUIRE_CALL(*rhs_expr, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .RETURN(Value::create(frst::Map{}));
        REQUIRE_CALL(*key_expr, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .RETURN(Value::create(Array{Value::create(1_f)}));
        FORBID_CALL(syms, define(_, _));

        std::vector<Map_Destructure::Element> elems;
        elems.emplace_back(
            Map_Destructure::Element{std::move(key_expr), "bar"});
        Map_Destructure node{std::move(elems), std::move(rhs_expr)};

        CHECK_THROWS_MATCHES(
            node.execute(syms), Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring(
                "Non-primitive key expressions are not permitted in Map "
                "destructuring:")));
    }

    SECTION("Key expression errors propagate and stop evaluation")
    {
        auto rhs_expr = mock::Mock_Expression::make();
        auto key_expr1 = mock::Mock_Expression::make();
        auto key_expr2 = mock::Mock_Expression::make();
        mock::Mock_Symbol_Table syms;

        REQUIRE_CALL(*rhs_expr, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .RETURN(Value::create(frst::Map{}));
        REQUIRE_CALL(*key_expr1, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .THROW(Frost_Recoverable_Error{"kaboom"});
        FORBID_CALL(*key_expr2, evaluate(_));
        FORBID_CALL(syms, define(_, _));

        std::vector<Map_Destructure::Element> elems;
        elems.emplace_back(Map_Destructure::Element{std::move(key_expr1), "a"});
        elems.emplace_back(Map_Destructure::Element{std::move(key_expr2), "b"});
        Map_Destructure node{std::move(elems), std::move(rhs_expr)};

        CHECK_THROWS_MATCHES(node.execute(syms), Frost_Recoverable_Error,
                             MessageMatches(Equals("kaboom")));
    }

    SECTION("Non-primitive key after earlier bind stops later binds")
    {
        auto rhs_expr = mock::Mock_Expression::make();
        auto key_expr1 = mock::Mock_Expression::make();
        auto key_expr2 = mock::Mock_Expression::make();
        auto key_expr3 = mock::Mock_Expression::make();
        mock::Mock_Symbol_Table syms;
        trompeloeil::sequence seq;

        auto value = Value::create(1_f);
        auto rhs_map = Value::create(frst::Map{{Value::create("ok"s), value}});

        REQUIRE_CALL(*rhs_expr, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(rhs_map);
        REQUIRE_CALL(*key_expr1, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(Value::create("ok"s));
        REQUIRE_CALL(syms, define("first", value)).IN_SEQUENCE(seq);
        REQUIRE_CALL(*key_expr2, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(Value::create(Array{Value::create(2_f)}));
        FORBID_CALL(*key_expr3, evaluate(_));
        FORBID_CALL(syms, define("third", _));

        std::vector<Map_Destructure::Element> elems;
        elems.emplace_back(
            Map_Destructure::Element{std::move(key_expr1), "first"});
        elems.emplace_back(
            Map_Destructure::Element{std::move(key_expr2), "second"});
        elems.emplace_back(
            Map_Destructure::Element{std::move(key_expr3), "third"});
        Map_Destructure node{std::move(elems), std::move(rhs_expr)};

        CHECK_THROWS_MATCHES(
            node.execute(syms), Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring(
                "Non-primitive key expressions are not permitted in Map "
                "destructuring:")));
    }

    SECTION("Primitive edge keys bind correctly")
    {
        auto rhs_expr = mock::Mock_Expression::make();
        auto key_null = mock::Mock_Expression::make();
        auto key_bool = mock::Mock_Expression::make();
        auto key_float = mock::Mock_Expression::make();
        mock::Mock_Symbol_Table syms;
        trompeloeil::sequence seq;

        auto v_null = Value::create("n"s);
        auto v_bool = Value::create("b"s);
        auto v_float = Value::create("f"s);
        auto rhs_map = Value::create(frst::Map{
            {Value::null(), v_null},
            {Value::create(true), v_bool},
            {Value::create(3.5), v_float},
        });

        REQUIRE_CALL(*rhs_expr, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(rhs_map);
        REQUIRE_CALL(*key_null, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(Value::null());
        REQUIRE_CALL(syms, define("n", v_null)).IN_SEQUENCE(seq);
        REQUIRE_CALL(*key_bool, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(Value::create(true));
        REQUIRE_CALL(syms, define("b", v_bool)).IN_SEQUENCE(seq);
        REQUIRE_CALL(*key_float, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(Value::create(3.5));
        REQUIRE_CALL(syms, define("f", v_float)).IN_SEQUENCE(seq);

        std::vector<Map_Destructure::Element> elems;
        elems.emplace_back(Map_Destructure::Element{std::move(key_null), "n"});
        elems.emplace_back(Map_Destructure::Element{std::move(key_bool), "b"});
        elems.emplace_back(Map_Destructure::Element{std::move(key_float), "f"});
        Map_Destructure node{std::move(elems), std::move(rhs_expr)};

        auto result = node.execute(syms);
        CHECK_FALSE(result.has_value());
    }

    SECTION("Duplicate key expressions are accepted")
    {
        auto rhs_expr = mock::Mock_Expression::make();
        auto key_expr1 = mock::Mock_Expression::make();
        auto key_expr2 = mock::Mock_Expression::make();
        mock::Mock_Symbol_Table syms;
        trompeloeil::sequence seq;

        auto value = Value::create(99_f);
        auto rhs_map = Value::create(frst::Map{{Value::create("dup"s), value}});
        auto key_val1 = Value::create("dup"s);
        auto key_val2 = Value::create("dup"s);

        REQUIRE_CALL(*rhs_expr, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(rhs_map);
        REQUIRE_CALL(*key_expr1, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(key_val1);
        REQUIRE_CALL(syms, define("a", value)).IN_SEQUENCE(seq);
        REQUIRE_CALL(*key_expr2, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(key_val2);
        REQUIRE_CALL(syms, define("b", value)).IN_SEQUENCE(seq);

        std::vector<Map_Destructure::Element> elems;
        elems.emplace_back(Map_Destructure::Element{std::move(key_expr1), "a"});
        elems.emplace_back(Map_Destructure::Element{std::move(key_expr2), "b"});
        Map_Destructure node{std::move(elems), std::move(rhs_expr)};

        auto result = node.execute(syms);
        CHECK_FALSE(result.has_value());
    }

    SECTION("Duplicate missing keys bind null to each name")
    {
        auto rhs_expr = mock::Mock_Expression::make();
        auto key_expr1 = mock::Mock_Expression::make();
        auto key_expr2 = mock::Mock_Expression::make();
        mock::Mock_Symbol_Table syms;
        trompeloeil::sequence seq;

        REQUIRE_CALL(*rhs_expr, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(Value::create(frst::Map{}));
        REQUIRE_CALL(*key_expr1, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(Value::create("missing"s));
        REQUIRE_CALL(syms, define("a", _))
            .IN_SEQUENCE(seq)
            .LR_WITH(_2->template is<Null>());
        REQUIRE_CALL(*key_expr2, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(Value::create("missing"s));
        REQUIRE_CALL(syms, define("b", _))
            .IN_SEQUENCE(seq)
            .LR_WITH(_2->template is<Null>());

        std::vector<Map_Destructure::Element> elems;
        elems.emplace_back(Map_Destructure::Element{std::move(key_expr1), "a"});
        elems.emplace_back(Map_Destructure::Element{std::move(key_expr2), "b"});
        Map_Destructure node{std::move(elems), std::move(rhs_expr)};

        auto result = node.execute(syms);
        CHECK_FALSE(result.has_value());
    }

    SECTION("Symbol sequence orders usages and definitions")
    {
        auto rhs_expr = std::make_unique<Sequence_Expression>(
            std::vector<Statement::Symbol_Action>{
                Statement::Usage{"rhs"},
            });
        auto key_expr1 = std::make_unique<Sequence_Expression>(
            std::vector<Statement::Symbol_Action>{
                Statement::Usage{"k1"},
            });
        auto key_expr2 = std::make_unique<Sequence_Expression>(
            std::vector<Statement::Symbol_Action>{
                Statement::Usage{"k2"},
                Statement::Usage{"k2b"},
            });

        std::vector<Map_Destructure::Element> elems;
        elems.emplace_back(Map_Destructure::Element{std::move(key_expr1), "a"});
        elems.emplace_back(Map_Destructure::Element{std::move(key_expr2), "b"});
        Map_Destructure node{std::move(elems), std::move(rhs_expr)};

        auto seq = collect_sequence(node);
        std::vector<std::string> expected{
            "use:rhs", "use:k1", "def:a", "use:k2", "use:k2b", "def:b",
        };
        CHECK(seq == expected);
    }
}
