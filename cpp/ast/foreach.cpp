#include <frost/ast/foreach.hpp>

#include <frost/backtrace.hpp>

using namespace frst;

ast::Foreach::Foreach(Source_Range source_range, Expression::Ptr structure,
                      Expression::Ptr operation)
    : Expression(source_range)
    , structure_{std::move(structure)}
    , operation_{std::move(operation)}
{
}

Value_Ptr ast::Foreach::do_evaluate(Evaluation_Context ctx) const
{
    const auto& structure_val = structure_->evaluate(ctx);
    if (not structure_val->is_structured())
    {
        throw Frost_Recoverable_Error{fmt::format(
            "Cannot iterate value with type {}", structure_val->type_name())};
    }

    const auto& op_val = operation_->evaluate(ctx);
    if (not op_val->is<Function>())
    {
        throw Frost_Recoverable_Error{
            fmt::format("Foreach operation expected Function, got {}",
                        op_val->type_name())};
    }

    const auto& op = op_val->raw_get<Function>();

    auto guard = make_frame_guard("Foreach ({})", op->name());

    if (structure_val->is<Array>())
    {
        const auto& arr = structure_val->raw_get<Array>();
        for (const auto& elem : arr)
        {
            if (op->call({elem})->truthy())
                break;
        }
        return Value::null();
    }

    if (structure_val->is<Map>())
    {
        const auto& map = structure_val->raw_get<Map>();
        for (const auto& [k, v] : map)
        {
            if (op->call({k, v})->truthy())
                break;
        }
        return Value::null();
    }

    THROW_UNREACHABLE;
}

std::generator<ast::AST_Node::Child_Info> ast::Foreach::children() const
{
    co_yield make_child(structure_, "Structure");
    co_yield make_child(operation_, "Operation");
}

std::string ast::Foreach::do_node_label() const
{
    return "Foreach_Expr";
}
