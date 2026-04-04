#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <catch2/trompeloeil.hpp>

#include <frost/mock/mock-destructure.hpp>
#include <frost/mock/mock-expression.hpp>
#include <frost/mock/mock-symbol-table.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/ast/define.hpp>
#include <frost/value.hpp>

#include <ranges>
#include <vector>

using namespace frst;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;

using trompeloeil::_;

TEST_CASE("Define")
{
    SECTION("Evaluates expression and passes result to destructure")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto expr = mock::Mock_Expression::make();
        auto dest = mock::Mock_Destructure::make();
        auto* expr_ptr = expr.get();
        auto* dest_ptr = dest.get();

        auto value = Value::create(42_f);

        trompeloeil::sequence seq;
        REQUIRE_CALL(*expr_ptr, do_evaluate(_))
            .IN_SEQUENCE(seq)
            .RETURN(value);
        REQUIRE_CALL(*dest_ptr, do_destructure(_, value))
            .IN_SEQUENCE(seq);

        ast::Define node{ast::AST_Node::no_range, std::move(dest),
                         std::move(expr)};
        node.execute(ctx);
    }

    SECTION("Expression error propagates, destructure not called")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto expr = mock::Mock_Expression::make();
        auto dest = mock::Mock_Destructure::make();
        auto* expr_ptr = expr.get();
        auto* dest_ptr = dest.get();

        REQUIRE_CALL(*expr_ptr, do_evaluate(_))
            .THROW(Frost_Recoverable_Error{"expr boom"});
        FORBID_CALL(*dest_ptr, do_destructure(_, _));

        ast::Define node{ast::AST_Node::no_range, std::move(dest),
                         std::move(expr)};

        CHECK_THROWS_WITH(node.execute(ctx),
                          ContainsSubstring("expr boom"));
    }

    SECTION("Destructure error propagates")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto expr = mock::Mock_Expression::make();
        auto dest = mock::Mock_Destructure::make();
        auto* expr_ptr = expr.get();
        auto* dest_ptr = dest.get();

        REQUIRE_CALL(*expr_ptr, do_evaluate(_))
            .RETURN(Value::create(42_f));
        REQUIRE_CALL(*dest_ptr, do_destructure(_, _))
            .THROW(Frost_Recoverable_Error{"dest boom"});

        ast::Define node{ast::AST_Node::no_range, std::move(dest),
                         std::move(expr)};

        CHECK_THROWS_WITH(node.execute(ctx),
                          ContainsSubstring("dest boom"));
    }

    SECTION("symbol_sequence chains expression then destructure")
    {
        auto expr = mock::Mock_Expression::make();
        auto dest = mock::Mock_Destructure::make();

        // Both mocks yield empty symbol sequences by default,
        // so the combined sequence is empty
        ast::Define node{ast::AST_Node::no_range, std::move(dest),
                         std::move(expr)};

        auto actions =
            node.symbol_sequence() | std::ranges::to<std::vector>();
        CHECK(actions.empty());
    }

    SECTION("children yields expression and destructure")
    {
        auto expr = mock::Mock_Expression::make();
        auto dest = mock::Mock_Destructure::make();

        ast::Define node{ast::AST_Node::no_range, std::move(dest),
                         std::move(expr)};

        auto kids =
            node.children() | std::ranges::to<std::vector>();
        REQUIRE(kids.size() == 2);
        CHECK(kids[0].label == "Expression");
        CHECK(kids[1].label == "Bindings");
    }

    SECTION("node_label")
    {
        auto expr = mock::Mock_Expression::make();
        auto dest = mock::Mock_Destructure::make();

        ast::Define node{ast::AST_Node::no_range, std::move(dest),
                         std::move(expr)};

        CHECK(node.node_label() == "Define");
    }
}
