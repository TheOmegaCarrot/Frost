#include <frost/ast/filter.hpp>

#include <frost/backtrace.hpp>
#include <frost/value.hpp>

using namespace frst;

ast::Filter::Filter(const Source_Range& source_range, Expression::Ptr structure,
                    Expression::Ptr operation)
    : Expression(source_range)
    , structure_{std::move(structure)}
    , operation_{std::move(operation)}
{
}

Value_Ptr ast::Filter::do_evaluate(Evaluation_Context ctx) const
{

    const auto& structure_val = structure_->evaluate(ctx);
    if (not structure_val->is_structured())
    {
        throw Frost_Recoverable_Error{fmt::format(
            "Cannot filter value with type {}", structure_val->type_name())};
    }

    const auto& op_val = operation_->evaluate(ctx);
    if (not op_val->is<Function>())
    {
        throw Frost_Recoverable_Error{fmt::format(
            "Filter operation expected Function, got {}", op_val->type_name())};
    }

    const auto& fn = op_val->raw_get<Function>();

    auto guard = make_frame_guard("Filter ({})", fn->name());

    return Value::do_filter(structure_val, fn);
}

std::generator<ast::AST_Node::Child_Info> ast::Filter::children() const
{
    co_yield make_child(structure_, "Structure");
    co_yield make_child(operation_, "Operation");
}

std::string ast::Filter::do_node_label() const
{
    return "Filter_Expr";
}
