#include <frost/ast/index.hpp>

#include <frost/value.hpp>

using namespace frst;
using namespace frst::ast;

Index::Index(Expression::Ptr structure, Expression::Ptr index)
    : structure_{std::move(structure)}
    , index_{std::move(index)}
{
}

static Value_Ptr index_array(const Array& array, const Value_Ptr& index_val)
{
    /*   -3 -2 -1
     *  [ a, b, c ]  The same indexing rules as Python
     *    0  1  2
     */

    if (!index_val->is<Int>())
        throw Frost_Recoverable_Error{"Cannot index array with non-integer"};

    const Int len = array.size();
    const Int index = index_val->raw_get<Int>();

    if (index >= 0 && index < len)
        return array.at(index);

    if (index < 0 && len + index >= 0)
        return array.at(index + len);

    return Value::null(); // out-of-bounds -> null
}

static Value_Ptr index_map(const Map& map, const Value_Ptr& key_val)
{
    if (auto itr = map.find(key_val); itr != map.end())
        return itr->second;

    return Value::null();
}

Value_Ptr Index::evaluate(const Symbol_Table& syms) const
{
    auto struct_val = structure_->evaluate(syms);
    if (!struct_val->is_structured())
    {
        throw Frost_Recoverable_Error{
            fmt::format("Cannot index type {}, expected structure",
                        struct_val->type_name())};
    }

    auto index_val = index_->evaluate(syms);

    if (struct_val->is<Array>())
        return index_array(struct_val->raw_get<Array>(), index_val);

    if (struct_val->is<Map>())
        return index_map(struct_val->raw_get<Map>(), index_val);

    THROW_UNREACHABLE;
}

std::string Index::node_label() const
{
    return "Index_Expression";
}

auto Index::children() const -> std::generator<Child_Info>
{
    co_yield make_child(structure_, "Structure");
    co_yield make_child(index_, "Index");
}
