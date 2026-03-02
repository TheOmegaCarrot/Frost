#include <frost/ast/foreach.hpp>

using namespace frst;

ast::Foreach::Foreach(Expression::Ptr structure, Expression::Ptr operation)
    : structure_{std::move(structure)}
    , operation_{std::move(operation)}
{
}

Value_Ptr ast::Foreach::evaluate(const Symbol_Table& syms) const
{
    const auto& structure_val = structure_->evaluate(syms);
    if (not structure_val->is_structured())
    {
        throw Frost_Recoverable_Error{fmt::format(
            "Cannot iterate value with type {}", structure_val->type_name())};
    }

    const auto& op_val = operation_->evaluate(syms);
    if (not op_val->is<Function>())
    {
        throw Frost_Recoverable_Error{
            fmt::format("Foreach operation expected Function, got {}",
                        op_val->type_name())};
    }

    const auto& op = op_val->raw_get<Function>();

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

std::generator<ast::Statement::Child_Info> ast::Foreach::children() const
{
    co_yield make_child(structure_, "Structure");
    co_yield make_child(operation_, "Operation");
}

std::string ast::Foreach::node_label() const
{
    return "Foreach_Expr";
}
