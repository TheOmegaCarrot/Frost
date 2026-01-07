#include <catch2/catch_test_macros.hpp>

#include <catch2/trompeloeil.hpp>

#include <frost/mock/mock-expression.hpp>
#include <frost/mock/mock-symbol-table.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/ast.hpp>

using namespace frst;
using namespace std::literals;

using trompeloeil::_;

TEST_CASE("Binop")
{
    auto lhs = std::make_unique<mock::Mock_Expression>();
    auto rhs = std::make_unique<mock::Mock_Expression>();
    mock::Mock_Symbol_Table syms;

    auto lhs_val = Value::create(42_f);
    auto rhs_val = Value::create(10_f);

    for (const std::string op : {"+", "-", "*", "/"})
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

            if (op == "+")
                CHECK(res->get<Int>() == 42_f + 10_f);
            if (op == "-")
                CHECK(res->get<Int>() == 42_f - 10_f);
            if (op == "*")
                CHECK(res->get<Int>() == 42_f * 10_f);
            if (op == "/")
                CHECK(res->get<Int>() == 42_f / 10_f);
            if (op == "and")
                CHECK(res->get<Int>() == 10_f);
            if (op == "or")
                CHECK(res->get<Int>() == 42_f);
        }
    }
}
