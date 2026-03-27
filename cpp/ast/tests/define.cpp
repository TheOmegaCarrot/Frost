#include <catch2/catch_test_macros.hpp>

#include <catch2/trompeloeil.hpp>

#include <frost/mock/mock-expression.hpp>
#include <frost/mock/mock-symbol-table.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/ast.hpp>
#include <frost/symbol-table.hpp>

using namespace frst;
using namespace std::literals;

using trompeloeil::_;

TEST_CASE("Define")
{
    auto expr = std::make_unique<mock::Mock_Expression>();
    mock::Mock_Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};
    trompeloeil::sequence seq;

    SECTION("Normal")
    {
        auto value = Value::create(42_f);

        REQUIRE_CALL(*expr, do_evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1.symbols == &syms)
            .RETURN(value);

        REQUIRE_CALL(syms, define("foo", value)).IN_SEQUENCE(seq);

        ast::Define node{ast::AST_Node::no_range, "foo", std::move(expr)};

        node.execute(ctx);
    }

    SECTION("Redefine")
    {
        auto value1 = Value::create(42_f);
        auto value2 = Value::create("well that's not right"s);

        REQUIRE_CALL(*expr, do_evaluate(_))
            .LR_WITH(&_1.symbols == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(value1);

        REQUIRE_CALL(syms, define("foo", value1)).IN_SEQUENCE(seq);

        REQUIRE_CALL(*expr, do_evaluate(_))
            .LR_WITH(&_1.symbols == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(value2);

        REQUIRE_CALL(syms, define("foo", value2))
            .IN_SEQUENCE(seq)
            .THROW(Frost_Recoverable_Error{"uh oh"});

        ast::Define node{ast::AST_Node::no_range, "foo", std::move(expr)};

        node.execute(ctx);
        CHECK_THROWS(node.execute(ctx));
    }

    SECTION("Export flag appears in symbol_sequence")
    {
        ast::Define node{ast::AST_Node::no_range, "foo", std::move(expr),
                         true};

        std::vector<ast::AST_Node::Symbol_Action> actions;
        for (const auto& action : node.symbol_sequence())
            actions.push_back(action);

        REQUIRE(actions.size() >= 1);
        auto& last = actions.back();
        REQUIRE(std::holds_alternative<ast::AST_Node::Definition>(last));
        auto& defn = std::get<ast::AST_Node::Definition>(last);
        CHECK(defn.name == "foo");
        CHECK(defn.exported == true);
    }

    SECTION("Non-export definition is not marked exported")
    {
        ast::Define node{ast::AST_Node::no_range, "foo", std::move(expr)};

        std::vector<ast::AST_Node::Symbol_Action> actions;
        for (const auto& action : node.symbol_sequence())
            actions.push_back(action);

        REQUIRE(actions.size() >= 1);
        auto& last = actions.back();
        REQUIRE(std::holds_alternative<ast::AST_Node::Definition>(last));
        auto& defn = std::get<ast::AST_Node::Definition>(last);
        CHECK(defn.name == "foo");
        CHECK(defn.exported == false);
    }

    SECTION("Reject _")
    {
        CHECK_THROWS(
            ast::Define{ast::AST_Node::no_range, "_", std::move(expr)});
    }
}
