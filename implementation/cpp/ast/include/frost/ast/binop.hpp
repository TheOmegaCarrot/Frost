#ifndef FROST_AST_BINOP_HPP
#define FROST_AST_BINOP_HPP

#include "expression.hpp"
#include "frost/symbol-table.hpp"

#include <fmt/format.h>

#include <string_view>

#include <frost/value.hpp>

namespace frst::ast
{

enum class Binary_Op
{
    PLUS,
    MINUS,
    MULTIPLY,
    DIVIDE,
    MODULUS,
    EQ,
    NE,
    LT,
    LE,
    GT,
    GE,
    AND,
    OR,
};

constexpr std::string_view format_binary_op(Binary_Op op)
{
    using enum Binary_Op;
    switch (op)
    {
    case PLUS:
        return "+";
    case MINUS:
        return "-";
    case MULTIPLY:
        return "*";
    case DIVIDE:
        return "/";
    case MODULUS:
        return "%";
    case EQ:
        return "==";
    case NE:
        return "!=";
    case LT:
        return "<";
    case LE:
        return "<=";
    case GT:
        return ">";
    case GE:
        return ">=";
    case AND:
        return "and";
    case OR:
        return "or";
    }
    THROW_UNREACHABLE;
}

class Binop final : public Expression
{

  public:
    using Ptr = std::unique_ptr<Binop>;

    Binop(Expression::Ptr lhs, Binary_Op op, Expression::Ptr rhs)
        : lhs_{std::move(lhs)}
        , rhs_{std::move(rhs)}
        , op_{op}
    {
    }

    Binop() = delete;
    Binop(const Binop&) = delete;
    Binop(Binop&&) = delete;
    Binop& operator=(const Binop&) = delete;
    Binop& operator=(Binop&&) = delete;
    ~Binop() override = default;

    [[nodiscard]] Value_Ptr evaluate(const Symbol_Table& syms) const final
    {
        using enum Binary_Op;
        static const std::map<Binary_Op, decltype(&Value::add)> fn_map{
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

  protected:
    std::string node_label() const final
    {
        return fmt::format("Binary({})", format_binary_op(op_));
    }

    std::generator<Child_Info> children() const final
    {
        co_yield make_child(lhs_, "LHS");
        co_yield make_child(rhs_, "RHS");
    }

  private:
    Expression::Ptr lhs_;
    Expression::Ptr rhs_;
    Binary_Op op_;
};
} // namespace frst::ast

#endif
