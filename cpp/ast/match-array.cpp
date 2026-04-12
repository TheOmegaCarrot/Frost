#include <frost/ast/match-array.hpp>

namespace frst::ast
{

std::generator<AST_Node::Symbol_Action> Match_Array::symbol_sequence() const
{
    for (const auto& pattern : subpatterns_)
        co_yield std::ranges::elements_of(pattern->symbol_sequence());

    if (rest_ && rest_->name)
        co_yield Definition{.name = rest_->name.value(), .exported = false};
}

std::generator<AST_Node::Child_Info> Match_Array::children() const
{
    for (const auto& pattern : subpatterns_)
        co_yield make_child(pattern);
}

bool Match_Array::do_try_match(Execution_Context ctx,
                               const Value_Ptr& value) const
{
    if (not value->is<Array>())
        return false;

    const auto& arr = value->raw_get<Array>();

    // Without a rest clause the sizes must match exactly; with one the
    // scrutinee may be longer than the element patterns.
    if (rest_)
    {
        if (arr.size() < subpatterns_.size())
            return false;
    }
    else
    {
        if (arr.size() != subpatterns_.size())
            return false;
    }

    for (const auto& [pat, elem] : std::views::zip(subpatterns_, arr))
    {
        if (not pat->try_match(ctx, elem))
            return false;
    }

    if (rest_ && rest_->name)
    {
        auto tail = arr
                    | std::views::drop(subpatterns_.size())
                    | std::ranges::to<Array>();

        ctx.symbols.define(rest_->name.value(),
                           Value::create(std::move(tail)));
    }

    return true;
}

std::string Match_Array::do_node_label() const
{
    if (rest_)
        return fmt::format("Match_Array(...{})", rest_->name.value_or("_"));
    return "Match_Array";
}

} // namespace frst::ast
