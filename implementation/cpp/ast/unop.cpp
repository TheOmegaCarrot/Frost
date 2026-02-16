#include <frost/ast/unop.hpp>

using namespace frst;

ast::Unop::Unop(Expression::Ptr operand, Unary_Op op)
    : operand_{std::move(operand)}
    , op_{op}
{
}

Value_Ptr ast::Unop::evaluate(const Symbol_Table& syms) const
{
    auto operand_value = operand_->evaluate(syms);
    switch (op_)
    {
        using enum Unary_Op;
    case NEGATE:
        return operand_value->negate();
    case NOT:
        return operand_value->logical_not();
    }
    THROW_UNREACHABLE;
}

std::string ast::Unop::node_label() const
{
    return fmt::format("Unary({})", format_unary_op(op_));
}

auto ast::Unop::children() const -> std::generator<Child_Info>
{
    co_yield make_child(operand_);
}
