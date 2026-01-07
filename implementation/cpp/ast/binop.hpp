#ifndef FROST_AST_BINOP_HPP
#define FROST_AST_BINOP_HPP

#include "expression.hpp"
#include "frost/symbol-table.hpp"

#include <fmt/format.h>

#include <frost/value.hpp>

namespace frst::ast
{

enum class Binary_Op
{
    PLUS,
    MINUS,
    TIMES,
    DIVIDE,
    AND,
    OR,
};

struct Convert_Binary_Op
{
    using enum Binary_Op;
    static std::optional<Binary_Op> operator()(const std::string& op)
    {
        if (op == "+")
            return PLUS;
        if (op == "-")
            return MINUS;
        if (op == "*")
            return TIMES;
        if (op == "/")
            return DIVIDE;

        return std::nullopt;
    }
    static std::string_view operator()(Binary_Op op)
    {
        switch (op)
        {
        case PLUS:
            return "+";
        case MINUS:
            return "-";
        case TIMES:
            return "*";
        case DIVIDE:
            return "/";
        case AND:
            return "and";
        case OR:
            return "or";
        }
        THROW_UNREACHABLE;
    }
} constexpr static convert_binary_op;

class Binop final : public Expression
{

  public:
    using Ptr = std::unique_ptr<Binop>;

    Binop(Expression::Ptr lhs, const std::string& op, Expression::Ptr rhs)
        : lhs_{std::move(lhs)}
        , rhs_{std::move(rhs)}
        , op_{convert_binary_op(op)
                  .or_else([&] -> std::optional<Binary_Op> {
                      throw Frost_Error{
                          fmt::format("Bad binary operator {}", op)};
                      return {};
                  })
                  .value()}
    {
    }

    Binop() = delete;
    Binop(const Binop&) = delete;
    Binop(Binop&&) = delete;
    Binop& operator=(const Binop&) = delete;
    Binop& operator=(Binop&&) = delete;
    ~Binop() override = default;

    Value_Ptr evaluate(const Symbol_Table& syms) const final
    {
        auto lhs_value = lhs_->evaluate(syms);
        auto rhs_value = rhs_->evaluate(syms);
        switch (op_)
        {
            using enum Binary_Op;
        case PLUS:
            return Value::add(lhs_value, rhs_value);
        case MINUS:
            return Value::subtract(lhs_value, rhs_value);
        case TIMES:
            return Value::multiply(lhs_value, rhs_value);
        case DIVIDE:
            return Value::divide(lhs_value, rhs_value);
        case AND:
            return Value::logical_and(lhs_value, rhs_value);
        case OR:
            return Value::logical_or(lhs_value, rhs_value);
        }
        THROW_UNREACHABLE;
    }

  protected:
    std::string node_label() const final
    {
        return fmt::format("Binary({})", convert_binary_op(op_));
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
