#include <catch2/catch_test_macros.hpp>

#include <catch2/trompeloeil.hpp>

#include "../mock/mock-expression.hpp"

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/ast.hpp>
#include <frost/symbol-table.hpp>

using namespace frst;
using namespace std::literals;

using trompeloeil::_;

TEST_CASE("If")
{
    auto condition = ast::mock::Mock_Expression::make();
    auto consequent = ast::mock::Mock_Expression::make();
    auto alternate = ast::mock::Mock_Expression::make();

    Symbol_Table syms;

    auto make_if = [&](bool with_alternate = true) {
        if (with_alternate)
        {
            return ast::If{std::move(condition), std::move(consequent),
                           std::move(alternate)};
        }
        else
        {
            return ast::If{std::move(condition), std::move(consequent)};
        }
    };

    SECTION("Consequent Taken")
    {
        REQUIRE_CALL(*condition, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .RETURN(Value::create(42_f));

        REQUIRE_CALL(*consequent, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .RETURN(Value::create("yay!"s));

        auto node = make_if();

        auto res = node.evaluate(syms);
        CHECK(res->get<String>() == "yay!");
    }

    SECTION("Alternate Taken")
    {
        REQUIRE_CALL(*condition, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .RETURN(Value::create(false));

        REQUIRE_CALL(*alternate, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .RETURN(Value::create("yay!"s));

        auto node = make_if();

        auto res = node.evaluate(syms);
        CHECK(res->get<String>() == "yay!");
    }

    SECTION("Null Alternate Taken")
    {
        REQUIRE_CALL(*condition, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .RETURN(Value::create(false));

        auto node = make_if(false);

        auto res = node.evaluate(syms);
        CHECK(res->is<Null>());
    }
}
