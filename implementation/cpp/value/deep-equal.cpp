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

                         std::vector<bool> matched(rhs.size(), false);

                         auto entries_match = [&](const auto& lhs_kv,
                                                  const auto& rhs_kv) {
                             const auto& [lhs_key, lhs_value] = lhs_kv;
                             const auto& [rhs_key, rhs_value] = rhs_kv;
                             return std::visit(recurse, lhs_key->value_,
                                               rhs_key->value_)
                                    && std::visit(recurse, lhs_value->value_,
                                                  rhs_value->value_);
                         };

                         auto rhs_indexed = std::views::enumerate(rhs);
                         auto find_match = [&](const auto& lhs_kv)
                             -> std::optional<std::size_t> {
                             auto it = std::ranges::find_if(
                                 rhs_indexed, [&](const auto& indexed) {
                                     const auto& [idx, rhs_kv] = indexed;
                                     auto i = static_cast<std::size_t>(idx);
                                     return !matched[i]
                                            && entries_match(lhs_kv, rhs_kv);
                                 });
                             if (it == std::ranges::end(rhs_indexed))
                                 return std::nullopt;
                             return static_cast<std::size_t>(std::get<0>(*it));
                         };

                         return std::ranges::all_of(
                             lhs, [&](const auto& lhs_kv) {
                                 if (auto idx = find_match(lhs_kv))
                                 {
                                     matched[*idx] = true;
                                     return true;
                                 }
                                 return false;
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
