#include <frost/ast/filter.hpp>

#include <frost/value.hpp>

using namespace frst;

ast::Filter::Filter(Expression::Ptr structure, Expression::Ptr operation)
    : structure_{std::move(structure)}
    , operation_{std::move(operation)}
{
}

Value_Ptr ast::Filter::evaluate(const Symbol_Table& syms) const
{

    const auto& structure_val = structure_->evaluate(syms);
    if (not structure_val->is_structured())
    {
        throw Frost_Recoverable_Error{fmt::format(
            "Cannot filter value with type {}", structure_val->type_name())};
    }

    const auto& op_val = operation_->evaluate(syms);
    if (not op_val->is<Function>())
    {
        throw Frost_Recoverable_Error{fmt::format(
            "Filter operation expected Function, got {}", op_val->type_name())};
    }

    return Value::do_filter(structure_val, op_val->raw_get<Function>());
}

std::generator<ast::Statement::Child_Info> ast::Filter::children() const
{
    co_yield make_child(structure_, "Structure");
    co_yield make_child(operation_, "Operation");
}

std::string ast::Filter::node_label() const
{
    return "Filter_Expr";
}
