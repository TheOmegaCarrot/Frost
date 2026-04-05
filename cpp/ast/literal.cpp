#include <frost/ast/literal.hpp>

using namespace frst;

ast::Literal::Literal(const Source_Range& source_range, Value_Ptr value)
    : Expression(source_range)
    , value_{std::move(value)}
{
    if (!value_->is_primitive())
    {
        throw Frost_Interpreter_Error{
            fmt::format("Literal AST node created with non-primitive type: {}",
                        value_->type_name())};
    }
}

Value_Ptr ast::Literal::do_evaluate(Evaluation_Context) const
{
    return value_;
}

std::string ast::Literal::do_node_label() const
{
    return fmt::format("Literal({})",
                       value_->to_internal_string({.in_structure = true}));
}

bool ast::Literal::data_safe() const
{
    return true;
}
