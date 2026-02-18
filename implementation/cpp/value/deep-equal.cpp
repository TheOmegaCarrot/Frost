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

                         for (const auto& [lhs_kv, rhs_kv] :
                              std::views::zip(lhs, rhs))
                         {
                             const auto& [lhs_k, lhs_v] = lhs_kv;
                             const auto& [rhs_k, rhs_v] = rhs_kv;

                             if (not std::visit(recurse, lhs_k->value_,
                                                rhs_k->value_)
                                 || not std::visit(recurse, lhs_v->value_,
                                                   rhs_v->value_))
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
