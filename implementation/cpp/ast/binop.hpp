#ifndef FROST_AST_BINOP_HPP
#define FROST_AST_BINOP_HPP

#include "expression.hpp"
#include "frost/symbol-table.hpp"

#include <fmt/format.h>

#include <frost/value.hpp>

namespace frst::ast
{

enum class Op
{
    PLUS,
    MINUS,
    TIMES,
    DIVIDE,
};

struct Convert_Op
{
    using enum Op;
    static std::optional<Op> operator()(char op)
    {
        switch (op)
        {
        case '+':
            return PLUS;
        case '-':
            return MINUS;
        case '*':
            return TIMES;
        case '/':
            return DIVIDE;
        }
        return std::nullopt;
    }
    static char operator()(Op op)
    {
        switch (op)
        {
        case PLUS:
            return '+';
        case MINUS:
            return '-';
        case TIMES:
            return '*';
        case DIVIDE:
            return '/';
        }
    }
} constexpr static convert_op;

class Binop final : public Expression
{

  public:
    using Ptr = std::unique_ptr<Expression>;

    Binop(Expression::Ptr lhs, char op_c, Expression::Ptr rhs)
        : lhs_{std::move(lhs)}
        , rhs_{std::move(rhs)}
        , op_{convert_op(op_c)
                  .or_else([&] -> std::optional<Op> {
                      throw Frost_Error{
                          fmt::format("Bad binary operator {}", op_c)};
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
            using enum Op;
        case PLUS:
            return Value::add(lhs_value, rhs_value);
        case MINUS:
            return Value::subtract(lhs_value, rhs_value);
        case TIMES:
            return Value::multiply(lhs_value, rhs_value);
        case DIVIDE:
            return Value::divide(lhs_value, rhs_value);
        }
    }

  protected:
    std::string node_label() const final
    {
        return fmt::format("Binary({})", convert_op(op_));
    }

    std::generator<Child_Info> children() const final
    {
        co_yield make_child(lhs_, "LHS");
        co_yield make_child(rhs_, "RHS");
    }

  private:
    Expression::Ptr lhs_;
    Expression::Ptr rhs_;
    Op op_;
};
} // namespace frst::ast

#endif
