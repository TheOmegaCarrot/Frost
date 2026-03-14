#include <frost/ast/unop.hpp>

using namespace frst;

ast::Unop::Unop(Source_Range source_range, Expression::Ptr operand, Unary_Op op)
    : Expression(source_range)
    , operand_{std::move(operand)}
    , op_{op}
{
}

Value_Ptr ast::Unop::do_evaluate(Evaluation_Context ctx) const
{
    auto operand_value = operand_->evaluate(ctx);
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

std::string ast::Unop::do_node_label() const
{
    return fmt::format("Unary({})", format_unary_op(op_));
}

std::generator<ast::Statement::Child_Info> ast::Unop::children() const
{
    co_yield make_child(operand_);
}

bool ast::Unop::data_safe() const
{
    return true;
}
