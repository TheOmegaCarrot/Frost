#include <frost/value.hpp>

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

                         std::vector<Map::const_iterator> rhs_entries;
                         rhs_entries.reserve(rhs.size());
                         for (auto it = rhs.begin(); it != rhs.end(); ++it)
                             rhs_entries.push_back(it);

                         std::vector<bool> matched(rhs_entries.size(), false);

                         for (const auto& [lhs_key, lhs_value] : lhs)
                         {
                             bool found_match = false;
                             for (std::size_t i = 0; i < rhs_entries.size();
                                  ++i)
                             {
                                 if (matched.at(i))
                                     continue;

                                 const auto& [rhs_key, rhs_value] =
                                     *rhs_entries.at(i);
                                 if (std::visit(recurse, lhs_key->value_,
                                                rhs_key->value_)
                                     && std::visit(recurse, lhs_value->value_,
                                                   rhs_value->value_))
                                 {
                                     matched.at(i) = true;
                                     found_match = true;
                                     break;
                                 }
                             }

                             if (!found_match)
                                 return false;
                         }

                         return true;
                     },
                     []<Frost_Type T, Frost_Type U>
                         requires(not std::same_as<T, U>)
        (const T&, const U&) {
            return false;
        }};

    return std::visit(visitor, lhs->value_, rhs->value_);
}

} // namespace frst
