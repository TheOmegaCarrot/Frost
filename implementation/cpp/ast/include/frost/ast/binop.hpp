#ifndef FROST_AST_BINOP_HPP
#define FROST_AST_BINOP_HPP

#include "expression.hpp"

#include <fmt/format.h>

#include <flat_map>
#include <string_view>

#include <frost/symbol-table.hpp>
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

    Binop(Expression::Ptr lhs, Binary_Op op, Expression::Ptr rhs);

    Binop() = delete;
    Binop(const Binop&) = delete;
    Binop(Binop&&) = delete;
    Binop& operator=(const Binop&) = delete;
    Binop& operator=(Binop&&) = delete;
    ~Binop() override = default;

    [[nodiscard]] Value_Ptr evaluate(const Symbol_Table& syms) const final;

  protected:
    std::string node_label() const final;

    std::generator<Child_Info> children() const final;

  private:
    Expression::Ptr lhs_;
    Expression::Ptr rhs_;
    Binary_Op op_;
};
} // namespace frst::ast

#endif
