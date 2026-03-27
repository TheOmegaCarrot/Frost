#include <frost/ast/binop.hpp>

using namespace frst;

ast::Binop::Binop(Source_Range source_range, Expression::Ptr lhs, Binary_Op op,
                  Expression::Ptr rhs)
    : Expression(source_range)
    , lhs_{std::move(lhs)}
    , rhs_{std::move(rhs)}
    , op_{op}
{
}

Value_Ptr ast::Binop::do_evaluate(Evaluation_Context ctx) const
{
    using enum Binary_Op;

    auto lhs_val = lhs_->evaluate(ctx);

    if (op_ == AND)
    {
        if (lhs_val->truthy())
            return rhs_->evaluate(ctx);
        else
            return lhs_val;
    }

    if (op_ == OR)
    {
        if (lhs_val->truthy())
            return lhs_val;
        else
            return rhs_->evaluate(ctx);
    }

    auto rhs_val = rhs_->evaluate(ctx);

#define X_BINOP_MAP                                                            \
    X(PLUS, Value::add)                                                        \
    X(MINUS, Value::subtract)                                                  \
    X(MULTIPLY, Value::multiply)                                               \
    X(DIVIDE, Value::divide)                                                   \
    X(MODULUS, Value::modulus)                                                 \
    X(EQ, Value::equal)                                                        \
    X(NE, Value::not_equal)                                                    \
    X(LT, Value::less_than)                                                    \
    X(LE, Value::less_than_or_equal)                                           \
    X(GT, Value::greater_than)                                                 \
    X(GE, Value::greater_than_or_equal)

    switch (op_)
    {

#define X(OP, FN)                                                              \
    case OP:                                                                   \
        return FN(lhs_val, rhs_val);

        X_BINOP_MAP

#undef X
    case AND:
        [[fallthrough]];
    case OR:
        THROW_UNREACHABLE;
    }

    THROW_UNREACHABLE;
}

std::string ast::Binop::do_node_label() const
{
    return fmt::format("Binary({})", format_binary_op(op_));
}

std::generator<ast::AST_Node::Child_Info> ast::Binop::children() const
{
    co_yield make_child(lhs_, "LHS");
    co_yield make_child(rhs_, "RHS");
}

bool ast::Binop::data_safe() const
{
    return true;
}
