#ifndef FROST_AST_UNOP_HPP
#define FROST_AST_UNOP_HPP

#include "expression.hpp"
#include "frost/symbol-table.hpp"

#include <fmt/format.h>

#include <frost/value.hpp>

namespace frst::ast
{

enum class Unary_Op
{
    NEGATE,
};

struct Convert_Unary_Op
{
    using enum Unary_Op;
    static std::optional<Unary_Op> operator()(char op)
    {
        switch (op)
        {
        case '-':
            return NEGATE;
        }
        return std::nullopt;
    }
    static char operator()(Unary_Op op)
    {
        switch (op)
        {
        case NEGATE:
            return '-';
        }
        THROW_UNREACHABLE;
    }
} constexpr static convert_unary_op;

class Unop final : public Expression
{

  public:
    using Ptr = std::unique_ptr<Unop>;

    Unop(Expression::Ptr operand, char op_c)
        : operand_{std::move(operand)}
        , op_{convert_unary_op(op_c)
                  .or_else([&] -> std::optional<Unary_Op> {
                      throw Frost_Error{
                          fmt::format("Bad unary operator {}", op_c)};
                      return {};
                  })
                  .value()}
    {
    }

    Unop() = delete;
    Unop(const Unop&) = delete;
    Unop(Unop&&) = delete;
    Unop& operator=(const Unop&) = delete;
    Unop& operator=(Unop&&) = delete;
    ~Unop() override = default;

    Value_Ptr evaluate(const Symbol_Table& syms) const final
    {
        auto operand_value = operand_->evaluate(syms);
        switch (op_)
        {
            using enum Unary_Op;
        case NEGATE:
            return operand_value->negate();
        }
        THROW_UNREACHABLE;
    }

  protected:
    std::string node_label() const final
    {
        return fmt::format("Unary({})", convert_unary_op(op_));
    }

    std::generator<Child_Info> children() const final
    {
        co_yield make_child(operand_);
    }

  private:
    Expression::Ptr operand_;
    Unary_Op op_;
};
} // namespace frst::ast

#endif
