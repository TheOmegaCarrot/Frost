#include <catch2/catch_test_macros.hpp>

#include <catch2/trompeloeil.hpp>

#include <frost/mock/mock-expression.hpp>
#include <frost/mock/mock-symbol-table.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/ast.hpp>

using namespace frst;
using namespace std::literals;

using trompeloeil::_;

TEST_CASE("Unop")
{
    auto operand = std::make_unique<mock::Mock_Expression>();
    mock::Mock_Symbol_Table syms;

    auto operand_val = Value::create(42_f);

    for (const auto op : {ast::Unary_Op::NEGATE, ast::Unary_Op::NOT})
    {
        DYNAMIC_SECTION("Operator " << ast::format_unary_op(op))
        {
            REQUIRE_CALL(*operand, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .RETURN(operand_val);

            ast::Unop node(std::move(operand), op);

            auto res = node.evaluate(syms);

            if (op == ast::Unary_Op::NEGATE)
                CHECK(res->get<Int>() == -42_f);
            if (op == ast::Unary_Op::NOT)
                CHECK(res->get<Bool>() == false);
        }
    }
}
