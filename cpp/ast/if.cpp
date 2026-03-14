#include <frost/ast/if.hpp>

using namespace frst;

ast::If::If(Expression::Ptr condition, Expression::Ptr consequent,
            std::optional<Expression::Ptr> alternate)
    : condition_{std::move(condition)}
    , consequent_{std::move(consequent)}
    , alternate_{std::move(alternate)}
{
}

Value_Ptr ast::If::do_evaluate(Evaluation_Context ctx) const
{
    if (condition_->evaluate(ctx)->truthy())
        return consequent_->evaluate(ctx);
    else if (alternate_.has_value())
        return (*alternate_)->evaluate(ctx);

    // If an if-expression has no alternate branch,
    // and the condition is false, then evaluate to null
    return Value::null();
}

std::string ast::If::node_label() const
{
    return "If";
}

std::generator<ast::Statement::Child_Info> ast::If::children() const
{
    co_yield make_child(condition_, "Condition");
    co_yield make_child(consequent_, "Consequent");
    if (alternate_)
        co_yield make_child(*alternate_, "Alternate");
}
