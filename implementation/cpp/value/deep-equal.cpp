#include <frost/value.hpp>

#include <ranges>

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

                         std::size_t equal_elements_seen{};
                         for (const auto& [lhp, rhp] :
                              std::views::cartesian_product(lhs, rhs))
                         {
                             if (std::visit(recurse, lhp.first->value_,
                                            rhp.first->value_)
                                 && std::visit(recurse, lhp.second->value_,
                                               rhp.second->value_))
                                 ++equal_elements_seen;
                         }

                         return equal_elements_seen == lhs.size();
                     },
                     []<Frost_Type T, Frost_Type U>
                         requires(not std::same_as<T, U>)
        (const T&, const U&) {
            return false;
        }};

    return std::visit(visitor, lhs->value_, rhs->value_);
}

} // namespace frst
