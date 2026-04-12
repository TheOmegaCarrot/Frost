#include <frost/ast/destructure-map.hpp>

namespace frst::ast
{

void Destructure_Map::do_destructure(Execution_Context ctx,
                                     const Value_Ptr& value) const
{
    if (not value->is<frst::Map>())
        throw Frost_Recoverable_Error{fmt::format(
            "Destructure expected Map, got {}", value->type_name())};

    const frst::Map& map_being_destructured = value->raw_get<frst::Map>();

    for (const auto& [key_expr, destructure_child] : destructure_elems_)
    {
        auto key = key_expr->evaluate(ctx.as_eval());
        if (not key->is_primitive() || key->is<Null>())
        {
            throw Frost_Recoverable_Error{
                fmt::format("Map destructure key expressions must be valid "
                            "Map keys, got: {}",
                            key->type_name())};
        }

        auto itr = map_being_destructured.find(key);

        if (itr != map_being_destructured.end())
        {
            destructure_child->destructure(ctx, itr->second);
        }
        else
        {
            throw Frost_Recoverable_Error{fmt::format(
                "Map destructure expected key {}, but was not found",
                key->to_internal_string())};
        }
    }
}

} // namespace frst::ast
