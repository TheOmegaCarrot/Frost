#include <frost/value.hpp>

#include <algorithm>
#include <ranges>
#include <vector>

namespace frst
{

bool Value::deep_equal_impl(const Value_Ptr& lhs, const Value_Ptr& rhs)
{
    if (lhs == rhs)
        return true;

    // clang-format really has no clue what to do with this one, eh?
    // I don't blame it, this is kinda ugly
    auto visitor = Overload{
        []<typename T>
            requires Frost_Primitive<T> || std::same_as<Function, T>
                     (const T& lhs, const T& rhs) {
                         return lhs == rhs;
                     },
                     [](this const auto& recurse, const Array& lhs,
                        const Array& rhs) {
                         if (lhs.size() != rhs.size())
                             return false;

                         for (const auto& [lhv, rhv] :
                              std::views::zip(lhs, rhs))
                         {
                             if (not std::visit(recurse, lhv->value_,
                                                rhv->value_))
                                 return false;
                         }

                         return true;
                     },
                     [](this const auto& recurse, const Map& lhs,
                        const Map& rhs) {
                         if (lhs.size() != rhs.size())
                             return false;

                         auto entries_match = [&](const auto& lhs_kv,
                                                  const auto& rhs_kv) {
                             const auto& [lhs_key, lhs_value] = lhs_kv;
                             const auto& [rhs_key, rhs_value] = rhs_kv;
                             return std::visit(recurse, lhs_key->value_,
                                               rhs_key->value_)
                                    && std::visit(recurse, lhs_value->value_,
                                                  rhs_value->value_);
                         };

                         std::vector<std::pair<Value_Ptr, Value_Ptr>>
                             rhs_entries{std::from_range, rhs};

                         return std::ranges::all_of(
                             lhs, [&](const auto& lhs_kv) {
                                 auto it = std::ranges::find_if(
                                     rhs_entries, [&](const auto& rhs_kv) {
                                         return entries_match(lhs_kv, rhs_kv);
                                     });
                                 if (it == rhs_entries.end())
                                     return false;
                                 rhs_entries.erase(it);
                                 return true;
                             });
                     },
                     []<Frost_Type T, Frost_Type U>
                         requires(not std::same_as<T, U>)
        (const T&, const U&) {
            return false;
        }};

    return std::visit(visitor, lhs->value_, rhs->value_);
}

} // namespace frst
