#ifndef FROST_AST_EXPRESSION_HPP
#define FROST_AST_EXPRESSION_HPP

#include "statement.hpp"

#include <frost/value.hpp>

namespace frst::ast
{
class Expression : public Statement
{
  public:
    using Ptr = std::unique_ptr<Expression>;

    Expression() = default;
    Expression(const Expression&) = delete;
    Expression(Expression&&) = delete;
    Expression& operator=(const Expression&) = delete;
    Expression& operator=(Expression&&) = delete;
    ~Expression() override = default;

    //! @brief Evaluate the expression, and get the value it evaluates to
    [[nodiscard]] Value_Ptr evaluate(const Symbol_Table& table) const
    {
        return do_evaluate(table);
    }

  protected:
    [[nodiscard]] virtual Value_Ptr do_evaluate(
        const Symbol_Table& table) const = 0;

    //! Executing an expression is simply to evaluate it and discard the result
    std::optional<Map> do_execute(Symbol_Table& table) const final
    {
        (void)evaluate(table);
        return std::nullopt;
    }
};
} // namespace frst::ast

#endif
