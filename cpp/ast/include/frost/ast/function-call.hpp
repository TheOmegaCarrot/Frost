#ifndef FROST_AST_FUNCTION_CALL_HPP
#define FROST_AST_FUNCTION_CALL_HPP

#include "expression.hpp"

#include <frost/value.hpp>

#include <fmt/format.h>

namespace frst::ast
{
class Function_Call final : public Expression
{
  public:
    using Ptr = std::unique_ptr<Function_Call>;

    Function_Call() = delete;
    Function_Call(const Function_Call&) = delete;
    Function_Call(Function_Call&&) = delete;
    Function_Call& operator=(const Function_Call&) = delete;
    Function_Call& operator=(Function_Call&&) = delete;
    ~Function_Call() final = default;

    Function_Call(Source_Range source_range, Expression::Ptr fn_expr,
                  std::vector<Expression::Ptr> args_exprs);

  protected:
    std::string do_node_label() const final;

    [[nodiscard]] Value_Ptr do_evaluate(Evaluation_Context ctx) const final;

    std::generator<Child_Info> children() const final;

  private:
    Expression::Ptr fn_expr_;
    std::vector<Expression::Ptr> args_exprs_;
};
} // namespace frst::ast
#endif
