#include <catch2/catch_test_macros.hpp>

#include <catch2/trompeloeil.hpp>

#include "../mock/mock-expression.hpp"

#include <frost/ast.hpp>
#include <frost/symbol-table.hpp>

using namespace frst;

using trompeloeil::_;

TEST_CASE("Define")
{
    auto expr = std::make_unique<ast::mock::Mock_Expression>();
    Symbol_Table syms;

    SECTION("Normal")
    {
        REQUIRE_CALL(*expr, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .RETURN(Value::create(42_f));

        ast::Define node{"foo", std::move(expr)};

        CHECK_THROWS(syms.lookup("foo"));
        node.execute(syms);
        CHECK(syms.lookup("foo")->get<Int>() == 42_f);
    }
}
