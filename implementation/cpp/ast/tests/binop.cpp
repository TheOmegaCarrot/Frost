#include <catch2/catch_test_macros.hpp>

#include <catch2/trompeloeil.hpp>

#include "../mock/mock-expression.hpp"

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/ast.hpp>
#include <frost/symbol-table.hpp>

using namespace frst;
using namespace std::literals;

using trompeloeil::_;

TEST_CASE("Binop")
{
    auto lhs = std::make_unique<ast::mock::Mock_Expression>();
    auto rhs = std::make_unique<ast::mock::Mock_Expression>();
    Symbol_Table syms;

    auto lhs_val = Value::create(42_f);
    auto rhs_val = Value::create(10_f);

    for (const char op : {'+', '-', '*', '/'})
    {
        DYNAMIC_SECTION("Operator " << op)
        {
            trompeloeil::sequence seq;

            REQUIRE_CALL(*lhs, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(lhs_val);

            REQUIRE_CALL(*rhs, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(rhs_val);

            ast::Binop node(std::move(lhs), op, std::move(rhs));

            auto res = node.evaluate(syms);

            switch (op)
            {
            case '+':
                CHECK(res->get<Int>() == 42_f + 10_f);
                break;
            case '-':
                CHECK(res->get<Int>() == 42_f - 10_f);
                break;
            case '*':
                CHECK(res->get<Int>() == 42_f * 10_f);
                break;
            case '/':
                CHECK(res->get<Int>() == 42_f / 10_f);
                break;
            }
        }
    }
}
