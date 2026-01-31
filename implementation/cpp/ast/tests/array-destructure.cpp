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

TEST_CASE("Array_Destructure")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).

    SECTION("Defines names and skips discard")
    {
        auto expr = std::make_unique<mock::Mock_Expression>();
        mock::Mock_Symbol_Table syms;
        trompeloeil::sequence seq;

        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto c = Value::create(3_f);
        auto arr = Value::create(Array{a, b, c});

        REQUIRE_CALL(*expr, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(arr);

        REQUIRE_CALL(syms, define("x", a)).IN_SEQUENCE(seq);
        REQUIRE_CALL(syms, define("y", c)).IN_SEQUENCE(seq);

        std::vector<Array_Destructure::Name> names{
            std::string{"x"},
            Discarded_Binding{},
            std::string{"y"},
        };

        Array_Destructure node{std::move(names), std::nullopt, std::move(expr)};

        auto result = node.execute(syms);
        CHECK_FALSE(result.has_value());
    }

    SECTION("Exports only bound names when enabled")
    {
        auto expr = std::make_unique<mock::Mock_Expression>();
        mock::Mock_Symbol_Table syms;
        trompeloeil::sequence seq;

        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto c = Value::create(3_f);
        auto arr = Value::create(Array{a, b, c});

        REQUIRE_CALL(*expr, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(arr);

        REQUIRE_CALL(syms, define("x", a)).IN_SEQUENCE(seq);
        REQUIRE_CALL(syms, define("rest", _)).IN_SEQUENCE(seq).LR_SIDE_EFFECT({
            auto val = _2;
            REQUIRE(val->template is<Array>());
            const auto& out = val->template raw_get<Array>();
            REQUIRE(out.size() == 1);
            CHECK(out.at(0) == c);
        });

        std::vector<Array_Destructure::Name> names{
            std::string{"x"},
            Discarded_Binding{},
        };

        Array_Destructure node{
            std::move(names),
            Array_Destructure::Name{std::string{"rest"}},
            std::move(expr),
            true,
        };

        auto result = node.execute(syms);
        REQUIRE(result.has_value());
        CHECK(result->size() == 2);

        CHECK(result->find(Value::create("_"s)) == result->end());

        auto it_x = result->find(Value::create("x"s));
        REQUIRE(it_x != result->end());
        CHECK(it_x->second == a);

        auto it_rest = result->find(Value::create("rest"s));
        REQUIRE(it_rest != result->end());
        REQUIRE(it_rest->second->template is<Array>());
        const auto& rest_out = it_rest->second->template raw_get<Array>();
        REQUIRE(rest_out.size() == 1);
        CHECK(rest_out.at(0) == c);
    }

    SECTION("Exports null values when enabled")
    {
        auto expr = std::make_unique<mock::Mock_Expression>();
        mock::Mock_Symbol_Table syms;
        trompeloeil::sequence seq;

        auto a = Value::null();
        auto arr = Value::create(Array{a});

        REQUIRE_CALL(*expr, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(arr);

        REQUIRE_CALL(syms, define("x", a)).IN_SEQUENCE(seq);

        std::vector<Array_Destructure::Name> names{
            std::string{"x"},
        };

        Array_Destructure node{std::move(names), std::nullopt, std::move(expr),
                               true};

        auto result = node.execute(syms);
        REQUIRE(result.has_value());
        CHECK(result->size() == 1);

        auto it = result->find(Value::create("x"s));
        REQUIRE(it != result->end());
        CHECK(it->second == a);
    }

    SECTION("Rest binds remaining elements")
    {
        auto expr = std::make_unique<mock::Mock_Expression>();
        mock::Mock_Symbol_Table syms;
        trompeloeil::sequence seq;

        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto c = Value::create(3_f);
        auto arr = Value::create(Array{a, b, c});

        REQUIRE_CALL(*expr, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(arr);

        REQUIRE_CALL(syms, define("head", a)).IN_SEQUENCE(seq);
        REQUIRE_CALL(syms, define("rest", _)).IN_SEQUENCE(seq).LR_SIDE_EFFECT({
            auto val = _2;
            REQUIRE(val->template is<Array>());
            const auto& out = val->template raw_get<Array>();
            REQUIRE(out.size() == 2);
            CHECK(out.at(0) == b);
            CHECK(out.at(1) == c);
        });

        std::vector<Array_Destructure::Name> names{
            std::string{"head"},
        };
        Array_Destructure node{std::move(names),
                               Array_Destructure::Name{std::string{"rest"}},
                               std::move(expr)};

        auto result = node.execute(syms);
        CHECK_FALSE(result.has_value());
    }

    SECTION("Rest binds empty array when no extra elements")
    {
        auto expr = std::make_unique<mock::Mock_Expression>();
        mock::Mock_Symbol_Table syms;
        trompeloeil::sequence seq;

        auto a = Value::create(1_f);
        auto arr = Value::create(Array{a});

        REQUIRE_CALL(*expr, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(arr);

        REQUIRE_CALL(syms, define("head", a)).IN_SEQUENCE(seq);
        REQUIRE_CALL(syms, define("rest", _)).IN_SEQUENCE(seq).LR_SIDE_EFFECT({
            auto val = _2;
            REQUIRE(val->template is<Array>());
            CHECK(val->template raw_get<Array>().empty());
        });

        std::vector<Array_Destructure::Name> names{
            std::string{"head"},
        };
        Array_Destructure node{std::move(names),
                               Array_Destructure::Name{std::string{"rest"}},
                               std::move(expr)};

        auto result = node.execute(syms);
        CHECK_FALSE(result.has_value());
    }

    SECTION("Discarded rest ignores extra elements")
    {
        auto expr = std::make_unique<mock::Mock_Expression>();
        mock::Mock_Symbol_Table syms;
        trompeloeil::sequence seq;

        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto c = Value::create(3_f);
        auto arr = Value::create(Array{a, b, c});

        REQUIRE_CALL(*expr, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(arr);

        REQUIRE_CALL(syms, define("head", a)).IN_SEQUENCE(seq);
        FORBID_CALL(syms, define("_", _));

        std::vector<Array_Destructure::Name> names{
            std::string{"head"},
        };
        Array_Destructure node{std::move(names),
                               Array_Destructure::Name{Discarded_Binding{}},
                               std::move(expr)};

        auto result = node.execute(syms);
        CHECK_FALSE(result.has_value());
    }

    SECTION("Rest with discarded positional")
    {
        auto expr = std::make_unique<mock::Mock_Expression>();
        mock::Mock_Symbol_Table syms;
        trompeloeil::sequence seq;

        auto v1 = Value::create(1_f);
        auto v2 = Value::create(2_f);
        auto v3 = Value::create(3_f);
        auto arr = Value::create(Array{v1, v2, v3});

        REQUIRE_CALL(*expr, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(arr);

        REQUIRE_CALL(syms, define("a", v2)).IN_SEQUENCE(seq);
        REQUIRE_CALL(syms, define("rest", _)).IN_SEQUENCE(seq).LR_SIDE_EFFECT({
            auto val = _2;
            REQUIRE(val->template is<Array>());
            const auto& out = val->template raw_get<Array>();
            REQUIRE(out.size() == 1);
            CHECK(out.at(0) == v3);
        });

        std::vector<Array_Destructure::Name> names{
            Discarded_Binding{},
            std::string{"a"},
        };
        Array_Destructure node{std::move(names),
                               Array_Destructure::Name{std::string{"rest"}},
                               std::move(expr)};

        auto result = node.execute(syms);
        CHECK_FALSE(result.has_value());
    }

    SECTION("Rest binds empty array when exact size")
    {
        auto expr = std::make_unique<mock::Mock_Expression>();
        mock::Mock_Symbol_Table syms;
        trompeloeil::sequence seq;

        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto arr = Value::create(Array{a, b});

        REQUIRE_CALL(*expr, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(arr);

        REQUIRE_CALL(syms, define("a", a)).IN_SEQUENCE(seq);
        REQUIRE_CALL(syms, define("b", b)).IN_SEQUENCE(seq);
        REQUIRE_CALL(syms, define("rest", _)).IN_SEQUENCE(seq).LR_SIDE_EFFECT({
            auto val = _2;
            REQUIRE(val->template is<Array>());
            CHECK(val->template raw_get<Array>().empty());
        });

        std::vector<Array_Destructure::Name> names{
            std::string{"a"},
            std::string{"b"},
        };
        Array_Destructure node{std::move(names),
                               Array_Destructure::Name{std::string{"rest"}},
                               std::move(expr)};

        CHECK_NOTHROW(node.execute(syms));
    }

    SECTION("Errors on insufficient elements")
    {
        auto expr = std::make_unique<mock::Mock_Expression>();
        mock::Mock_Symbol_Table syms;

        auto arr = Value::create(Array{Value::create(1_f)});

        REQUIRE_CALL(*expr, evaluate(_)).LR_WITH(&_1 == &syms).RETURN(arr);
        FORBID_CALL(syms, define(_, _));

        std::vector<Array_Destructure::Name> names{
            std::string{"a"},
            std::string{"b"},
        };
        Array_Destructure node{std::move(names), std::nullopt, std::move(expr)};

        CHECK_THROWS_MATCHES(
            node.execute(syms), Frost_Recoverable_Error,
            MessageMatches(Equals("Insfficient array elements to destructure: "
                                  "required 2 but got 1")));
    }

    SECTION("Errors on insufficient elements even with rest")
    {
        auto expr = std::make_unique<mock::Mock_Expression>();
        mock::Mock_Symbol_Table syms;

        auto arr = Value::create(Array{Value::create(1_f)});

        REQUIRE_CALL(*expr, evaluate(_)).LR_WITH(&_1 == &syms).RETURN(arr);
        FORBID_CALL(syms, define(_, _));

        std::vector<Array_Destructure::Name> names{
            std::string{"a"},
            std::string{"b"},
        };
        Array_Destructure node{std::move(names),
                               Array_Destructure::Name{std::string{"rest"}},
                               std::move(expr)};

        CHECK_THROWS_MATCHES(
            node.execute(syms), Frost_Recoverable_Error,
            MessageMatches(Equals("Insfficient array elements to destructure: "
                                  "required 2 but got 1")));
    }

    SECTION("Errors on insufficient elements with discarded rest")
    {
        auto expr = std::make_unique<mock::Mock_Expression>();
        mock::Mock_Symbol_Table syms;

        auto arr = Value::create(Array{Value::create(1_f)});

        REQUIRE_CALL(*expr, evaluate(_)).LR_WITH(&_1 == &syms).RETURN(arr);
        FORBID_CALL(syms, define(_, _));

        std::vector<Array_Destructure::Name> names{
            std::string{"a"},
            std::string{"b"},
        };
        Array_Destructure node{std::move(names),
                               Array_Destructure::Name{Discarded_Binding{}},
                               std::move(expr)};

        CHECK_THROWS_MATCHES(
            node.execute(syms), Frost_Recoverable_Error,
            MessageMatches(Equals("Insfficient array elements to destructure: "
                                  "required 2 but got 1")));
    }

    SECTION("Errors on too many elements without rest")
    {
        auto expr = std::make_unique<mock::Mock_Expression>();
        mock::Mock_Symbol_Table syms;

        auto arr = Value::create(Array{Value::create(1_f), Value::create(2_f)});

        REQUIRE_CALL(*expr, evaluate(_)).LR_WITH(&_1 == &syms).RETURN(arr);
        FORBID_CALL(syms, define(_, _));

        std::vector<Array_Destructure::Name> names{
            std::string{"a"},
        };
        Array_Destructure node{std::move(names), std::nullopt, std::move(expr)};

        CHECK_THROWS_MATCHES(
            node.execute(syms), Frost_Recoverable_Error,
            MessageMatches(Equals("Too many array elements to destructure: "
                                  "required 1 but got 2")));
    }

    SECTION("Empty pattern succeeds on empty array")
    {
        auto expr = std::make_unique<mock::Mock_Expression>();
        mock::Mock_Symbol_Table syms;

        auto arr = Value::create(Array{});

        REQUIRE_CALL(*expr, evaluate(_)).LR_WITH(&_1 == &syms).RETURN(arr);
        FORBID_CALL(syms, define(_, _));

        std::vector<Array_Destructure::Name> names{};
        Array_Destructure node{std::move(names), std::nullopt, std::move(expr)};

        CHECK_NOTHROW(node.execute(syms));
    }

    SECTION("Empty pattern errors on non-empty array")
    {
        auto expr = std::make_unique<mock::Mock_Expression>();
        mock::Mock_Symbol_Table syms;

        auto arr = Value::create(Array{Value::create(1_f)});

        REQUIRE_CALL(*expr, evaluate(_)).LR_WITH(&_1 == &syms).RETURN(arr);
        FORBID_CALL(syms, define(_, _));

        std::vector<Array_Destructure::Name> names{};
        Array_Destructure node{std::move(names), std::nullopt, std::move(expr)};

        CHECK_THROWS_MATCHES(
            node.execute(syms), Frost_Recoverable_Error,
            MessageMatches(Equals("Too many array elements to destructure: "
                                  "required 0 but got 1")));
    }

    SECTION("Rest-only binds all elements")
    {
        auto expr = std::make_unique<mock::Mock_Expression>();
        mock::Mock_Symbol_Table syms;
        trompeloeil::sequence seq;

        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto arr = Value::create(Array{a, b});

        REQUIRE_CALL(*expr, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(arr);

        REQUIRE_CALL(syms, define("rest", _)).IN_SEQUENCE(seq).LR_SIDE_EFFECT({
            auto val = _2;
            REQUIRE(val->template is<Array>());
            const auto& out = val->template raw_get<Array>();
            REQUIRE(out.size() == 2);
            CHECK(out.at(0) == a);
            CHECK(out.at(1) == b);
        });

        std::vector<Array_Destructure::Name> names{};
        Array_Destructure node{std::move(names),
                               Array_Destructure::Name{std::string{"rest"}},
                               std::move(expr)};

        CHECK_NOTHROW(node.execute(syms));
    }

    SECTION("Rest-only binds empty array")
    {
        auto expr = std::make_unique<mock::Mock_Expression>();
        mock::Mock_Symbol_Table syms;
        trompeloeil::sequence seq;

        auto arr = Value::create(Array{});

        REQUIRE_CALL(*expr, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(arr);

        REQUIRE_CALL(syms, define("rest", _)).IN_SEQUENCE(seq).LR_SIDE_EFFECT({
            auto val = _2;
            REQUIRE(val->template is<Array>());
            CHECK(val->template raw_get<Array>().empty());
        });

        std::vector<Array_Destructure::Name> names{};
        Array_Destructure node{std::move(names),
                               Array_Destructure::Name{std::string{"rest"}},
                               std::move(expr)};

        CHECK_NOTHROW(node.execute(syms));
    }

    SECTION("Discarded rest with exact size")
    {
        auto expr = std::make_unique<mock::Mock_Expression>();
        mock::Mock_Symbol_Table syms;
        trompeloeil::sequence seq;

        auto a = Value::create(1_f);
        auto arr = Value::create(Array{a});

        REQUIRE_CALL(*expr, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(arr);

        REQUIRE_CALL(syms, define("head", a)).IN_SEQUENCE(seq);
        FORBID_CALL(syms, define("_", _));

        std::vector<Array_Destructure::Name> names{
            std::string{"head"},
        };
        Array_Destructure node{std::move(names),
                               Array_Destructure::Name{Discarded_Binding{}},
                               std::move(expr)};

        CHECK_NOTHROW(node.execute(syms));
    }

    SECTION("Discarded rest only ignores all elements")
    {
        auto expr = std::make_unique<mock::Mock_Expression>();
        mock::Mock_Symbol_Table syms;

        auto arr = Value::create(Array{Value::create(1_f), Value::create(2_f)});

        REQUIRE_CALL(*expr, evaluate(_)).LR_WITH(&_1 == &syms).RETURN(arr);
        FORBID_CALL(syms, define(_, _));

        std::vector<Array_Destructure::Name> names{};
        Array_Destructure node{std::move(names),
                               Array_Destructure::Name{Discarded_Binding{}},
                               std::move(expr)};

        CHECK_NOTHROW(node.execute(syms));
    }

    SECTION("Rest with discarded positional binds empty when exact size")
    {
        auto expr = std::make_unique<mock::Mock_Expression>();
        mock::Mock_Symbol_Table syms;
        trompeloeil::sequence seq;

        auto v1 = Value::create(1_f);
        auto v2 = Value::create(2_f);
        auto arr = Value::create(Array{v1, v2});

        REQUIRE_CALL(*expr, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(arr);

        REQUIRE_CALL(syms, define("a", v2)).IN_SEQUENCE(seq);
        REQUIRE_CALL(syms, define("rest", _)).IN_SEQUENCE(seq).LR_SIDE_EFFECT({
            auto val = _2;
            REQUIRE(val->template is<Array>());
            CHECK(val->template raw_get<Array>().empty());
        });

        std::vector<Array_Destructure::Name> names{
            Discarded_Binding{},
            std::string{"a"},
        };
        Array_Destructure node{std::move(names),
                               Array_Destructure::Name{std::string{"rest"}},
                               std::move(expr)};

        CHECK_NOTHROW(node.execute(syms));
    }

    SECTION("Errors on non-array RHS")
    {
        auto expr = std::make_unique<mock::Mock_Expression>();
        mock::Mock_Symbol_Table syms;

        REQUIRE_CALL(*expr, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .RETURN(Value::create(1_f));
        FORBID_CALL(syms, define(_, _));

        std::vector<Array_Destructure::Name> names{
            std::string{"a"},
        };
        Array_Destructure node{std::move(names), std::nullopt, std::move(expr)};

        CHECK_THROWS_MATCHES(
            node.execute(syms), Frost_Recoverable_Error,
            MessageMatches(Equals("Cannot destructure Int to Array")));
    }

    SECTION("RHS evaluate errors propagate without definitions")
    {
        auto expr = std::make_unique<mock::Mock_Expression>();
        mock::Mock_Symbol_Table syms;

        REQUIRE_CALL(*expr, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .THROW(Frost_Recoverable_Error{"kaboom"});
        FORBID_CALL(syms, define(_, _));

        std::vector<Array_Destructure::Name> names{
            std::string{"a"},
        };
        Array_Destructure node{std::move(names), std::nullopt, std::move(expr)};

        CHECK_THROWS_MATCHES(node.execute(syms), Frost_Recoverable_Error,
                             MessageMatches(Equals("kaboom")));
    }

    SECTION("Non-array RHS prevents any definitions even with rest")
    {
        auto expr = std::make_unique<mock::Mock_Expression>();
        mock::Mock_Symbol_Table syms;

        REQUIRE_CALL(*expr, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .RETURN(Value::create(1_f));
        FORBID_CALL(syms, define(_, _));

        std::vector<Array_Destructure::Name> names{
            std::string{"a"},
        };
        Array_Destructure node{std::move(names),
                               Array_Destructure::Name{std::string{"rest"}},
                               std::move(expr)};

        CHECK_THROWS_MATCHES(
            node.execute(syms), Frost_Recoverable_Error,
            MessageMatches(Equals("Cannot destructure Int to Array")));
    }

    SECTION("Duplicate names are rejected")
    {
        std::vector<Array_Destructure::Name> names{
            std::string{"dup"},
            std::string{"dup"},
        };

        CHECK_THROWS_MATCHES(
            (Array_Destructure{std::move(names), std::nullopt,
                               std::make_unique<mock::Mock_Expression>()}),
            Frost_Unrecoverable_Error,
            MessageMatches(
                Equals("Duplicate destructuring binding name: dup")));
    }

    SECTION("Duplicate name between rest and positional is rejected")
    {
        std::vector<Array_Destructure::Name> names{
            std::string{"dup"},
        };

        CHECK_THROWS_MATCHES(
            (Array_Destructure{std::move(names),
                               Array_Destructure::Name{std::string{"dup"}},
                               std::make_unique<mock::Mock_Expression>()}),
            Frost_Unrecoverable_Error,
            MessageMatches(
                Equals("Duplicate destructuring binding name: dup")));
    }

    SECTION("Duplicate positional names still error with discarded rest")
    {
        std::vector<Array_Destructure::Name> names{
            std::string{"dup"},
            std::string{"dup"},
        };

        CHECK_THROWS_MATCHES(
            (Array_Destructure{std::move(names),
                               Array_Destructure::Name{Discarded_Binding{}},
                               std::make_unique<mock::Mock_Expression>()}),
            Frost_Unrecoverable_Error,
            MessageMatches(
                Equals("Duplicate destructuring binding name: dup")));
    }

    SECTION("Duplicate discarded bindings are allowed")
    {
        auto expr = std::make_unique<mock::Mock_Expression>();
        mock::Mock_Symbol_Table syms;

        auto arr = Value::create(Array{Value::create(1_f), Value::create(2_f)});

        REQUIRE_CALL(*expr, evaluate(_)).LR_WITH(&_1 == &syms).RETURN(arr);
        FORBID_CALL(syms, define(_, _));

        std::vector<Array_Destructure::Name> names{
            Discarded_Binding{},
            Discarded_Binding{},
        };
        Array_Destructure node{std::move(names), std::nullopt, std::move(expr)};

        CHECK_NOTHROW(node.execute(syms));
    }

    SECTION("Symbol sequence includes RHS then definitions")
    {
        auto expr = std::make_unique<Sequence_Expression>(
            std::vector<Statement::Symbol_Action>{
                Statement::Usage{"rhs"},
            });

        std::vector<Array_Destructure::Name> names{
            std::string{"a"},
            Discarded_Binding{},
            std::string{"b"},
        };
        Array_Destructure node{std::move(names),
                               Array_Destructure::Name{std::string{"rest"}},
                               std::move(expr)};

        CHECK(collect_sequence(node)
              == std::vector<std::string>{"use:rhs", "def:a", "def:b",
                                          "def:rest"});
    }

    SECTION("Symbol sequence omits discarded rest")
    {
        auto expr = std::make_unique<Sequence_Expression>(
            std::vector<Statement::Symbol_Action>{
                Statement::Usage{"rhs"},
            });

        std::vector<Array_Destructure::Name> names{
            std::string{"a"},
        };
        Array_Destructure node{std::move(names),
                               Array_Destructure::Name{Discarded_Binding{}},
                               std::move(expr)};

        CHECK(collect_sequence(node)
              == std::vector<std::string>{"use:rhs", "def:a"});
    }

    SECTION("Symbol sequence with discarded rest and positional names")
    {
        auto expr = std::make_unique<Sequence_Expression>(
            std::vector<Statement::Symbol_Action>{
                Statement::Usage{"rhs"},
            });

        std::vector<Array_Destructure::Name> names{
            std::string{"a"},
            Discarded_Binding{},
            std::string{"b"},
        };
        Array_Destructure node{std::move(names),
                               Array_Destructure::Name{Discarded_Binding{}},
                               std::move(expr)};

        CHECK(collect_sequence(node)
              == std::vector<std::string>{"use:rhs", "def:a", "def:b"});
    }

    SECTION("Symbol sequence omits discarded positional bindings")
    {
        auto expr = std::make_unique<Sequence_Expression>(
            std::vector<Statement::Symbol_Action>{
                Statement::Usage{"rhs"},
            });

        std::vector<Array_Destructure::Name> names{
            Discarded_Binding{},
            std::string{"a"},
            Discarded_Binding{},
            std::string{"b"},
        };
        Array_Destructure node{std::move(names), std::nullopt, std::move(expr)};

        CHECK(collect_sequence(node)
              == std::vector<std::string>{"use:rhs", "def:a", "def:b"});
    }

    SECTION("Symbol sequence for rest-only")
    {
        auto expr = std::make_unique<Sequence_Expression>(
            std::vector<Statement::Symbol_Action>{
                Statement::Usage{"rhs"},
            });

        std::vector<Array_Destructure::Name> names{};
        Array_Destructure node{std::move(names),
                               Array_Destructure::Name{std::string{"rest"}},
                               std::move(expr)};

        CHECK(collect_sequence(node)
              == std::vector<std::string>{"use:rhs", "def:rest"});
    }

    SECTION("Symbol sequence for discarded rest-only")
    {
        auto expr = std::make_unique<Sequence_Expression>(
            std::vector<Statement::Symbol_Action>{
                Statement::Usage{"rhs"},
            });

        std::vector<Array_Destructure::Name> names{};
        Array_Destructure node{std::move(names),
                               Array_Destructure::Name{Discarded_Binding{}},
                               std::move(expr)};

        CHECK(collect_sequence(node) == std::vector<std::string>{"use:rhs"});
    }

    SECTION("Symbol sequence for empty pattern")
    {
        auto expr = std::make_unique<Sequence_Expression>(
            std::vector<Statement::Symbol_Action>{
                Statement::Usage{"rhs"},
            });

        std::vector<Array_Destructure::Name> names{};
        Array_Destructure node{std::move(names), std::nullopt, std::move(expr)};

        CHECK(collect_sequence(node) == std::vector<std::string>{"use:rhs"});
    }
}
