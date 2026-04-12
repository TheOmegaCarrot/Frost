#include <frost/ast/match-value.hpp>

namespace frst::ast
{

bool Match_Value::do_try_match(Execution_Context ctx,
                               const Value_Ptr& value) const
{
    return Value::equal(value, expr_->evaluate(ctx.as_eval()))->truthy();
}

std::string Match_Value::do_node_label() const
{
    return "Match_Value";
}

std::generator<AST_Node::Child_Info> Match_Value::children() const
{
    co_yield make_child(expr_);
}

} // namespace frst::ast
