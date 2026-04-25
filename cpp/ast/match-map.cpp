#include <frost/ast/match-map.hpp>

namespace frst::ast
{

std::generator<AST_Node::Symbol_Action> Match_Map::symbol_sequence() const
{
    for (const auto& [key_expr, pattern] : elements_)
    {
        co_yield std::ranges::elements_of(key_expr->symbol_sequence());
        co_yield std::ranges::elements_of(pattern->symbol_sequence());
    }

    if (bind_whole_name_)
        co_yield Definition{.name = bind_whole_name_.value(),
                            .exported = false};
}

std::generator<AST_Node::Child_Info> Match_Map::children() const
{
    for (const auto& [i, elem] : std::views::enumerate(elements_))
    {
        const auto& [key_expr, pattern] = elem;
        co_yield make_child(key_expr, fmt::format("Key {}", i + 1));
        co_yield make_child(pattern, fmt::format("Pattern {}", i + 1));
    }
}

bool Match_Map::do_try_match(Execution_Context ctx,
                             const Value_Ptr& value) const
{
    if (not value->is<frst::Map>())
        return false;

    const frst::Map& match_target = value->raw_get<frst::Map>();

    for (const auto& [key_expr, pattern] : elements_)
    {
        auto key = key_expr->evaluate(ctx.as_eval());
        if (key->visit([]<typename T>(const T&) {
                return not Frost_Map_Key<T>;
            }))
        {
            throw Frost_Recoverable_Error{
                fmt::format("Map match key expressions must be valid Map keys, "
                            "got: {}",
                            key->type_name())};
        }

        auto itr = match_target.find(key);

        if (itr == match_target.end())
            return false;

        if (not pattern->try_match(ctx, itr->second))
            return false;
    }

    if (bind_whole_name_)
        ctx.symbols.define(bind_whole_name_.value(), value);

    return true;
}

std::string Match_Map::do_node_label() const
{
    return fmt::format("Match_Map{}",
                       bind_whole_name_
                           ? fmt::format("(as {})", bind_whole_name_.value())
                           : "");
}

} // namespace frst::ast
