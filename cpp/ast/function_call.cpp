#include <frost/ast/function_call.hpp>

#include <ranges>

using namespace frst;

ast::Function_Call::Function_Call(Expression::Ptr fn_expr,
                                  std::vector<Expression::Ptr> args_exprs)
    : fn_expr_{std::move(fn_expr)}
    , args_exprs_{std::move(args_exprs)}
{
}

Value_Ptr ast::Function_Call::evaluate(const Symbol_Table& syms) const
{
    const auto& fn = fn_expr_->evaluate(syms);

    if (not fn->is<Function>())
    {
        throw Frost_Recoverable_Error{
            fmt::format("Cannot call value of type {}", fn->type_name())};
    }

    const auto args =
        args_exprs_
        | std::views::transform([&](const Expression::Ptr& arg_expr) {
              return arg_expr->evaluate(syms);
          })
        | std::ranges::to<std::vector>();

    return fn->raw_get<Function>()->call(std::move(args));
}

std::string ast::Function_Call::node_label() const
{
    return "Function_Call";
}

std::generator<ast::Statement::Child_Info> ast::Function_Call::children() const
{
    co_yield make_child(fn_expr_, "Function");
    for (const auto& [i, arg] : std::views::enumerate(args_exprs_))
        co_yield make_child(arg, fmt::format("Argument({})", i));
}
