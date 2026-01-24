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
    trompeloeil::sequence seq;

    SECTION("Normal")
    {
        auto value = Value::create(42_f);

        REQUIRE_CALL(*expr, evaluate(_))
            .IN_SEQUENCE(seq)
            .LR_WITH(&_1 == &syms)
            .RETURN(value);

        REQUIRE_CALL(syms, define("foo", value)).IN_SEQUENCE(seq);

        ast::Define node{"foo", std::move(expr)};

        CHECK_NOTHROW(node.execute(syms));
    }

    SECTION("Redefine")
    {
        auto value1 = Value::create(42_f);
        auto value2 = Value::create("well that's not right"s);

        REQUIRE_CALL(*expr, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(value1);

        REQUIRE_CALL(syms, define("foo", value1)).IN_SEQUENCE(seq);

        REQUIRE_CALL(*expr, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(value2);

        REQUIRE_CALL(syms, define("foo", value2))
            .IN_SEQUENCE(seq)
            .THROW(Frost_Recoverable_Error{"uh oh"});

        ast::Define node{"foo", std::move(expr)};

        CHECK_NOTHROW(node.execute(syms));
        CHECK_THROWS(node.execute(syms));
    }

    SECTION("Reject _")
    {
        CHECK_THROWS(ast::Define{"_", std::move(expr)});
    }
}
