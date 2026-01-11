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

    Function_Call(Expression::Ptr fn_expr,
                  std::vector<Expression::Ptr> args_exprs)
        : fn_expr_{std::move(fn_expr)}
        , args_exprs_{std::move(args_exprs)}
    {
    }

    [[nodiscard]] Value_Ptr evaluate(const Symbol_Table& syms) const final
    {
        const auto& fn = fn_expr_->evaluate(syms);

        if (not fn->is<Function>())
        {
            throw Frost_Error{
                fmt::format("Cannot call value of type {}", fn->type_name())};
        }

        const auto args =
            args_exprs_
            | std::views::transform([&](const Expression::Ptr& arg_expr) {
                  return arg_expr->evaluate(syms);
              })
            | std::ranges::to<std::vector>();

        [[gnu::musttail]] return fn->raw_get<Function>()->call(std::move(args));
    }

  protected:
    std::string node_label() const final
    {
        return "Function Call";
    }

    std::generator<Child_Info> children() const final
    {
        co_yield make_child(fn_expr_, "Function");
        for (const auto& [i, arg] : std::views::enumerate(args_exprs_))
            co_yield make_child(arg, fmt::format("Argument({})", i));
    }

  private:
    Expression::Ptr fn_expr_;
    std::vector<Expression::Ptr> args_exprs_;
};
} // namespace frst::ast
#endif
