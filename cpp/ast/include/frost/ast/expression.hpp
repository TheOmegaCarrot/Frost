#ifndef FROST_AST_EXPRESSION_HPP
#define FROST_AST_EXPRESSION_HPP

#include "statement.hpp"

#include <frost/backtrace.hpp>
#include <frost/value.hpp>

namespace frst::ast
{
class Expression : public Statement
{
  public:
    using Ptr = std::unique_ptr<Expression>;

    Expression(Source_Range source_range)
        : Statement(source_range)
    {
    }

    Expression() = delete;
    Expression(const Expression&) = delete;
    Expression(Expression&&) = delete;
    Expression& operator=(const Expression&) = delete;
    Expression& operator=(Expression&&) = delete;
    ~Expression() override = default;

    //! @brief Evaluate the expression, and get the value it evaluates to
    [[nodiscard]] Value_Ptr evaluate(Evaluation_Context ctx) const
    {
        auto* bt = Backtrace_State::current();
        if (not bt)
            return do_evaluate(ctx);

        Frame_Guard guard{bt, AST_Frame{.node = this}};
        return do_evaluate(ctx);
    }

  protected:
    [[nodiscard]] virtual Value_Ptr do_evaluate(
        Evaluation_Context ctx) const = 0;

    //! Executing an expression is simply to evaluate it and discard the result
    std::optional<Map> do_execute(Execution_Context& ctx) const final
    {
        (void)evaluate(ctx.as_eval());
        return std::nullopt;
    }
};
} // namespace frst::ast

#endif
