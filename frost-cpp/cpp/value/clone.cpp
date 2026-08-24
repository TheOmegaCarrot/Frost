#include <frost/value.hpp>

#include <ranges>

using namespace frst;

Value_Ptr Value::clone() const
{
    return value_.visit(
        Overload{[](const Null&) {
                     return Value::null();
                 },
                 [](const Frost_Primitive auto& value) {
                     return Value::create(auto{value});
                 },
                 [](const Function& value) {
                     return Value::create(auto{value});
                 },
                 [](const Array& value) {
                     return Value::create(
                         value
                         | std::views::transform([](const Value_Ptr& elem) {
                               return elem->clone();
                           })
                         | std::ranges::to<Array>());
                 },
                 [](const Map& value) {
                     Map acc;
                     for (const auto& [k, v] : value)
                     {
                         acc.emplace(k->clone(), v->clone());
                     }
                     return Value::create(Value::trusted, std::move(acc));
                 }});
}
