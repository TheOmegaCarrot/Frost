#include <frost/ast/reduce.hpp>

#include <frost/backtrace.hpp>
#include <frost/value.hpp>

using namespace frst;

ast::Reduce::Reduce(Source_Range source_range, Expression::Ptr structure,
                    Expression::Ptr operation,
                    std::optional<Expression::Ptr> init)
    : Expression(source_range)
    , structure_{std::move(structure)}
    , operation_{std::move(operation)}
    , init_{std::move(init)}
{
}

Value_Ptr ast::Reduce::do_evaluate(Evaluation_Context ctx) const
{
    const auto& structure_val = structure_->evaluate(ctx);
    if (not structure_val->is_structured())
    {
        throw Frost_Recoverable_Error{fmt::format(
            "Cannot reduce value with type {}", structure_val->type_name())};
    }

    const auto& op_val = operation_->evaluate(ctx);
    if (not op_val->is<Function>())
    {
        throw Frost_Recoverable_Error{fmt::format(
            "Reduce operation expected Function, got {}", op_val->type_name())};
    }

    const auto& fn = op_val->raw_get<Function>();

    auto init = init_.transform([&](const Expression::Ptr& expr) {
        return expr->evaluate(ctx);
    });

    Frame_Guard guard{ctx.runtime.backtrace,
                      Iterative_Frame{.operation = "Reduce",
                                      .function_name = fn->name()}};

    return Value::do_reduce(structure_val, fn, init);
}

std::generator<ast::Statement::Child_Info> ast::Reduce::children() const
{
    co_yield make_child(structure_, "Structure");
    co_yield make_child(operation_, "Operation");
    if (init_)
        co_yield make_child(*init_, "Init");
}

std::string ast::Reduce::do_node_label() const
{
    return "Reduce_Expr";
}
