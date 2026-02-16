#include <frost/ast/if.hpp>

using namespace frst;

ast::If::If(Expression::Ptr condition, Expression::Ptr consequent,
            std::optional<Expression::Ptr> alternate)
    : condition_{std::move(condition)}
    , consequent_{std::move(consequent)}
    , alternate_{std::move(alternate)}
{
}

Value_Ptr ast::If::evaluate(const Symbol_Table& syms) const
{
    if (condition_->evaluate(syms)->truthy())
        return consequent_->evaluate(syms);
    else if (alternate_.has_value())
        return (*alternate_)->evaluate(syms);

    // If an if-expression has no alternate branch,
    // and the condition is false, then evaluate to null
    return Value::null();
}

std::string ast::If::node_label() const
{
    return "If";
}

auto ast::If::children() const -> std::generator<Child_Info>
{
    co_yield make_child(condition_, "Condition");
    co_yield make_child(consequent_, "Consequent");
    if (alternate_)
        co_yield make_child(*alternate_, "Alternate");
}
