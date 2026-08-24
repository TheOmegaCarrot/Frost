#include <frost/ast/map-constructor.hpp>

using namespace frst;

ast::Map_Constructor::Map_Constructor(const Source_Range& source_range,
                                      std::vector<KV_Pair> pairs)
    : Expression(source_range)
    , pairs_{std::move(pairs)}
{
}

Value_Ptr ast::Map_Constructor::do_evaluate(Evaluation_Context ctx) const
{
    Map acc;

    for (const auto& [key_expr, value_expr] : pairs_)
    {
        auto key = key_expr->evaluate(ctx);
        auto value = value_expr->evaluate(ctx);
        acc.insert_or_assign(key, value);
    }

    return Value::create(std::move(acc));
}

std::string ast::Map_Constructor::do_node_label() const
{
    return "Map_Constructor";
}

std::generator<ast::AST_Node::Child_Info> ast::Map_Constructor::children() const
{
    for (const auto& [k, v] : pairs_)
    {
        co_yield make_child(k, "Key");
        co_yield make_child(v, "Value");
    }
}

bool ast::Map_Constructor::data_safe() const
{
    return true;
}
