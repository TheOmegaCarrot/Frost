#include "filter.hpp"
#include "frost/symbol-table.hpp"

#include <frost/value.hpp>

using namespace frst;

namespace
{

Value_Ptr filter_map(const Map& map, const Function& pred)
{
    Map acc;

    for (const auto& [k, v] : map)
    {
        if (pred->call({k, v})->as<Bool>().value())
            acc.insert({k, v});
    }

    return Value::create(std::move(acc));
}

Value_Ptr filter_array(const Array& arr, const Function& pred)
{
    Array acc;

    for (const auto& elem : arr)
    {
        if (pred->call({elem})->as<Bool>().value())
            acc.push_back(elem);
    }

    return Value::create(std::move(acc));
}

} // namespace

Value_Ptr ast::Filter::evaluate(const Symbol_Table& syms) const
{

    const auto& structure_val = structure_->evaluate(syms);
    if (not structure_val->is_structured())
    {
        throw Frost_Recoverable_Error{fmt::format(
            "Cannot filter value with type {}", structure_val->type_name())};
    }

    const auto& op_val = operation_->evaluate(syms);
    if (not op_val->is<Function>())
    {
        throw Frost_Recoverable_Error{fmt::format(
            "Filter operation expected Function, got {}", op_val->type_name())};
    }

    if (structure_val->is<Array>())
        return filter_array(structure_val->raw_get<Array>(),
                            op_val->raw_get<Function>());

    if (structure_val->is<Map>())
        return filter_map(structure_val->raw_get<Map>(),
                          op_val->raw_get<Function>());

    THROW_UNREACHABLE;
}
