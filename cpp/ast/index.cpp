#include <frost/ast/index.hpp>

#include <frost/value.hpp>

using namespace frst;
using namespace frst::ast;

Index::Index(Source_Range source_range, Expression::Ptr structure,
             Expression::Ptr index)
    : Expression(source_range)
    , structure_{std::move(structure)}
    , index_{std::move(index)}
{
}

template <typename T>
void throw_map_index_type_error()
{
    throw Frost_Recoverable_Error{
        fmt::format("Invalid type for Map index: {}", type_str<T>())};
}

static Value_Ptr index_map(const Map& map, const Value_Ptr& key_val)
{
    key_val->visit(Overload{[]<Frost_Type T>(const T&) {
                                throw_map_index_type_error<T>();
                            },
                            [](const Frost_Map_Key auto&) {
                            }});

    if (auto itr = map.find(key_val); itr != map.end())
        return itr->second;

    return Value::null();
}

Value_Ptr Index::do_evaluate(Evaluation_Context ctx) const
{
    auto struct_val = structure_->evaluate(ctx);
    if (!struct_val->is_structured())
    {
        throw Frost_Recoverable_Error{
            fmt::format("Cannot index type {}, expected structure",
                        struct_val->type_name())};
    }

    auto index_val = index_->evaluate(ctx);

    if (struct_val->is<Array>())
    {
        if (not index_val->is<Int>())
            throw Frost_Recoverable_Error{fmt::format(
                "Array index requires Int, got: {}", index_val->type_name())};

        return Value::index_array(struct_val->raw_get<Array>(),
                                  index_val->raw_get<Int>())
            .value_or(Value::null());
    }

    if (struct_val->is<Map>())
        return index_map(struct_val->raw_get<Map>(), index_val);

    THROW_UNREACHABLE;
}

std::string Index::do_node_label() const
{
    return "Index_Expression";
}

std::generator<AST_Node::Child_Info> Index::children() const
{
    co_yield make_child(structure_, "Structure");
    co_yield make_child(index_, "Index");
}
