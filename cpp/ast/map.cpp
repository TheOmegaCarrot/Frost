#include <ranges>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <frost/ast/map.hpp>

using namespace frst;

ast::Map::Map(Expression::Ptr structure, Expression::Ptr operation)
    : structure_{std::move(structure)}
    , operation_{std::move(operation)}
{
}

Value_Ptr ast::Map::evaluate(const Symbol_Table& syms) const
{
    const auto& structure_val = structure_->evaluate(syms);
    if (not structure_val->is_structured())
    {
        throw Frost_Recoverable_Error{fmt::format(
            "Cannot map value with type {}", structure_val->type_name())};
    }

    const auto& op_val = operation_->evaluate(syms);
    if (not op_val->is<Function>())
    {
        throw Frost_Recoverable_Error{fmt::format(
            "Map operation expected Function, got {}", op_val->type_name())};
    }

    return Value::do_map(structure_val, op_val->raw_get<Function>(), "Map");
}

std::generator<ast::Statement::Child_Info> ast::Map::children() const
{
    co_yield make_child(structure_, "Structure");
    co_yield make_child(operation_, "Operation");
}

std::string ast::Map::node_label() const
{
    return "Map_Expr";
}
