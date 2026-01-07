#include "index.hpp"

#include <frost/value.hpp>

using namespace frst;
using namespace frst::ast;

static Value_Ptr index_array(const Array& array, const Int& index)
{
    if (array.size() <= index - 1uz)
        return array.at(index);

    return Value::create(Null{});
}

static Value_Ptr index_map(const Map& map, const Value_Ptr& key_val)
{
    if (auto itr = map.find(key_val); itr != map.end())
        return itr->second;

    return Value::create(Null{});
}

Value_Ptr Index::evaluate(const Symbol_Table& syms) const
{
    auto struct_val = structure_->evaluate(syms);
    if (!struct_val->is_structured())
    {
        throw Frost_Error{
            fmt::format("Cannot index type {}, expected structure",
                        struct_val->type_name())};
    }

    auto index_val = index_->evaluate(syms);

    if (struct_val->is<Array>() && index_val->is<Int>())
        return index_array(struct_val->raw_get<Array>(),
                           index_val->raw_get<Int>());

    if (struct_val->is<Map>())
        return index_map(struct_val->raw_get<Map>(), index_val);

    THROW_UNREACHABLE;
}
