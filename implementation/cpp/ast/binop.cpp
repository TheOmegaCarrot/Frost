#include <frost/ast/binop.hpp>

using namespace frst;

ast::Binop::Binop(Expression::Ptr lhs, Binary_Op op, Expression::Ptr rhs)
    : lhs_{std::move(lhs)}
    , rhs_{std::move(rhs)}
    , op_{op}
{
}

Value_Ptr ast::Binop::evaluate(const Symbol_Table& syms) const
{
    using enum Binary_Op;
    static const std::flat_map<Binary_Op, decltype(&Value::add)> fn_map{
        {PLUS, &Value::add},
        {MINUS, &Value::subtract},
        {MULTIPLY, &Value::multiply},
        {DIVIDE, &Value::divide},
        {MODULUS, &Value::modulus},
        {EQ, &Value::equal},
        {NE, &Value::not_equal},
        {LT, &Value::less_than},
        {LE, &Value::less_than_or_equal},
        {GT, &Value::greater_than},
        {GE, &Value::greater_than_or_equal},
    };

    auto lhs_val = lhs_->evaluate(syms);

    if (auto itr = fn_map.find(op_); itr != fn_map.end())
    {
        auto rhs_val = rhs_->evaluate(syms);
        return itr->second(lhs_val, rhs_val);
    }

    if (op_ == AND)
    {
        if (lhs_val->truthy())
            return rhs_->evaluate(syms);
        else
            return lhs_val;
    }

    if (op_ == OR)
    {
        if (lhs_val->truthy())
            return lhs_val;
        else
            return rhs_->evaluate(syms);
    }

    THROW_UNREACHABLE;
}

std::string ast::Binop::node_label() const
{
    return fmt::format("Binary({})", format_binary_op(op_));
}

std::generator<ast::Statement::Child_Info> ast::Binop::children() const
{
    co_yield make_child(lhs_, "LHS");
    co_yield make_child(rhs_, "RHS");
}
