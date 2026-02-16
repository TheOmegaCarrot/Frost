#include <frost/ast/map-constructor.hpp>

using namespace frst;

ast::Map_Constructor::Map_Constructor(std::vector<KV_Pair> pairs)
    : pairs_{std::move(pairs)}
{
}

Value_Ptr ast::Map_Constructor::evaluate(const Symbol_Table& syms) const
{
    Map acc;

    for (const auto& [key_expr, value_expr] : pairs_)
    {
        auto key = key_expr->evaluate(syms);
        auto value = value_expr->evaluate(syms);
        acc.insert_or_assign(key, value);
    }

    return Value::create(std::move(acc));
}

std::string ast::Map_Constructor::node_label() const
{
    return "Map_Constructor";
}

auto ast::Map_Constructor::children() const -> std::generator<Child_Info>
{
    for (const auto& [k, v] : pairs_)
    {
        co_yield make_child(k, "Key");
        co_yield make_child(v, "Value");
    }
}
