#include <frost/ast/literal.hpp>

using namespace frst;

ast::Literal::Literal(Value_Ptr value)
    : value_{std::move(value)}
{
    if (!value_->is_primitive())
    {
        throw Frost_Internal_Error{
            fmt::format("Literal AST node created with non-primitive type: {}",
                        value_->type_name())};
    }
}

Value_Ptr ast::Literal::evaluate(const Symbol_Table&) const
{
    return value_;
}

std::string ast::Literal::node_label() const
{
    return fmt::format("Literal({})",
                       value_->to_internal_string({.in_structure = true}));
}
