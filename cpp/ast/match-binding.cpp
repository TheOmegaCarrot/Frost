#include <frost/ast/match-binding.hpp>

namespace frst::ast::TC
{

bool satisfies(std::optional<Type_Constraint> constraint,
               const Value_Ptr& value)
{
    if (not constraint)
        return true;

    return value->visit([&]<typename Type>(const Type&) {
        switch (constraint.value())
        {
        case Type_Constraint::Null:
            return std::same_as<Null, Type>;
        case Type_Constraint::Int:
            return std::same_as<Int, Type>;
        case Type_Constraint::Float:
            return std::same_as<Float, Type>;
        case Type_Constraint::Bool:
            return std::same_as<Bool, Type>;
        case Type_Constraint::String:
            return std::same_as<String, Type>;
        case Type_Constraint::Array:
            return std::same_as<Array, Type>;
        case Type_Constraint::Map:
            return std::same_as<frst::Map, Type>;
        case Type_Constraint::Function:
            return std::same_as<Function, Type>;
        case Type_Constraint::Primitive:
            return Frost_Primitive<Type>;
        case Type_Constraint::Numeric:
            return Frost_Numeric<Type>;
        case Type_Constraint::Structured:
            return Frost_Structured<Type>;
        case Type_Constraint::Nonnull:
            return not std::same_as<Null, Type>;
        }
        THROW_UNREACHABLE;
    });
}

} // namespace frst::ast::TC

namespace frst::ast
{

std::generator<AST_Node::Symbol_Action> Match_Binding::symbol_sequence() const
{
    if (name_)
        co_yield Definition{.name = name_.value(), .exported = false};
}

bool Match_Binding::do_try_match(Execution_Context ctx,
                                 const Value_Ptr& value) const
{
    if (not TC::satisfies(type_constraint_, value))
        return false;

    if (name_)
        ctx.symbols.define(name_.value(), value);

    return true;
}

std::string Match_Binding::do_node_label() const
{
    if (type_constraint_)
        return fmt::format("Match_Binding({} is {})", name_.value_or("_"),
                           type_constraint_.value());
    else
        return fmt::format("Match_Binding({})", name_.value_or("_"));
}

} // namespace frst::ast
