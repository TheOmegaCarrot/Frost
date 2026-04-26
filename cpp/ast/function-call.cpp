#include <frost/ast/function-call.hpp>
#include <frost/backtrace.hpp>

#include <ranges>

using namespace frst;

ast::Function_Call::Function_Call(const Source_Range& source_range,
                                  Expression::Ptr fn_expr,
                                  std::vector<Expression::Ptr> args_exprs)
    : Expression(source_range)
    , fn_expr_{std::move(fn_expr)}
    , args_exprs_{std::move(args_exprs)}
{
}

Value_Ptr ast::Function_Call::do_evaluate(Evaluation_Context ctx) const
{
    const auto& fn = fn_expr_->evaluate(ctx);

    if (not fn->is<Function>())
    {
        throw Frost_Recoverable_Error{
            fmt::format("Cannot call value of type {}", fn->type_name())};
    }

    const auto args =
        args_exprs_
        | std::views::transform([&](const Expression::Ptr& arg_expr) {
              return arg_expr->evaluate(ctx);
          })
        | std::ranges::to<std::vector>();

    const auto& callable = fn->raw_get<Function>();

    auto guard = make_frame_guard("In {}", callable->name());

    return callable->call(std::move(args));
}

std::string ast::Function_Call::do_node_label() const
{
    return "Function_Call";
}

std::generator<ast::AST_Node::Child_Info> ast::Function_Call::children() const
{
    co_yield make_child(fn_expr_, "Function");
    for (const auto& [i, arg] : std::views::enumerate(args_exprs_))
        co_yield make_child(arg, fmt::format("Argument({})", i));
}
