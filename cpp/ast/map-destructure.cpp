#include <frost/ast/map-destructure.hpp>

#include <flat_set>

#include <ranges>

using namespace frst;
using namespace frst::ast;

Map_Destructure::Map_Destructure(Source_Range source_range,
                                 std::vector<Element> destructure_elems,
                                 Expression::Ptr expr, bool export_defs)
    : Statement(source_range)
    , destructure_elems_{std::move(destructure_elems)}
    , expr_{std::move(expr)}
    , export_defs_{export_defs}
{
    std::flat_set<std::string_view> binding_names;

    for (const auto& [_, name] : destructure_elems_)
    {
        if (binding_names.contains(name))
        {
            throw Frost_Unrecoverable_Error{
                fmt::format("Duplicate destructuring binding name: {}", name)};
        }

        binding_names.insert(name);
    }
}

void Map_Destructure::do_execute(Execution_Context& ctx) const
{
    Value_Ptr expr_result = expr_->evaluate(ctx.as_eval());

    if (not expr_result->is<Map>())
    {
        throw Frost_Recoverable_Error{fmt::format(
            "Cannot destructure {} to Map", expr_result->type_name())};
    }

    const auto& map = expr_result->raw_get<Map>();

    for (const auto& [key_expr, name] : destructure_elems_)
    {
        auto key = key_expr->evaluate(ctx.as_eval());
        if (not key->is_primitive() || key->is<Null>())
        {
            throw Frost_Recoverable_Error(
                fmt::format("Map destructure key expressions must be valid Map "
                            "keys, got: {}",
                            key->to_internal_string()));
        }
        auto itr = map.find(key);
        if (itr == map.end())
            ctx.symbols.define(name, Value::null());
        else
            ctx.symbols.define(name, itr->second);
    }
}

std::string Map_Destructure::do_node_label() const
{
    return fmt::format("{}Map_Destructure", export_defs_ ? "Export_" : "");
}

std::generator<AST_Node::Child_Info> Map_Destructure::children() const
{
    for (const auto& [key, name] : destructure_elems_)
        co_yield make_child(key, fmt::format("Bind({})", name));
    co_yield make_child(expr_, "RHS");
}

std::generator<AST_Node::Symbol_Action> Map_Destructure::symbol_sequence() const
{
    co_yield std::ranges::elements_of(expr_->symbol_sequence());

    for (const auto& [key_expr, name] : destructure_elems_)
    {
        co_yield std::ranges::elements_of(key_expr->symbol_sequence());
        co_yield Definition{.name = name, .exported = export_defs_};
    }
}
