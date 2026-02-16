#ifndef FROST_AST_UNOP_HPP
#define FROST_AST_UNOP_HPP

#include "expression.hpp"
#include "frost/symbol-table.hpp"

#include <fmt/format.h>

#include <string_view>

#include <frost/value.hpp>

namespace frst::ast
{

enum class Unary_Op
{
    NEGATE,
    NOT,
};

constexpr std::string_view format_unary_op(Unary_Op op)
{
    using enum Unary_Op;
    switch (op)
    {
    case NEGATE:
        return "-";
    case NOT:
        return "not";
    }
    THROW_UNREACHABLE;
}

class Unop final : public Expression
{

  public:
    using Ptr = std::unique_ptr<Unop>;

    Unop(Expression::Ptr operand, Unary_Op op);

    Unop() = delete;
    Unop(const Unop&) = delete;
    Unop(Unop&&) = delete;
    Unop& operator=(const Unop&) = delete;
    Unop& operator=(Unop&&) = delete;
    ~Unop() override = default;

    Value_Ptr evaluate(const Symbol_Table& syms) const final;

  protected:
    std::string node_label() const final;

    std::generator<Child_Info> children() const final;

  private:
    Expression::Ptr operand_;
    Unary_Op op_;
};
} // namespace frst::ast

#endif
