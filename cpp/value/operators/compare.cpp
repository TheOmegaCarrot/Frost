#include <frost/value.hpp>

#include "operators-common.hpp"

namespace frst
{

[[noreturn]] void compare_err(std::string_view op_glyph,
                              std::string_view lhs_type,
                              std::string_view rhs_type)
{
    op_err("compare", op_glyph, lhs_type, rhs_type);
}

bool Value::equal_impl(const Value_Ptr& lhs, const Value_Ptr& rhs)
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

bool Value::not_equal_impl(const Value_Ptr& lhs, const Value_Ptr& rhs)
{
    return !equal_impl(lhs, rhs);
}

#define DEF_ORDERING(NAME, OP)                                                 \
    struct raw_##NAME##_fn_t                                                   \
    {                                                                          \
        static bool operator()(const Frost_Primitive auto& lhs,                \
                               const Frost_Primitive auto& rhs)                \
            requires requires {                                                \
                { lhs OP rhs } -> std::same_as<bool>;                          \
            }                                                                  \
        {                                                                      \
            return lhs OP rhs;                                                 \
        }                                                                      \
        static bool operator()(const auto&, const auto&)                       \
        {                                                                      \
            THROW_UNREACHABLE;                                                 \
        }                                                                      \
    } constexpr static raw_##NAME##_fn;                                        \
                                                                               \
    bool raw_##NAME(const auto& lhs, const auto& rhs)                          \
    {                                                                          \
        return std::visit(raw_##NAME##_fn, lhs, rhs);                          \
    }                                                                          \
                                                                               \
    bool Value::NAME##_impl(const Value_Ptr& lhs, const Value_Ptr& rhs)        \
    {                                                                          \
        if ((lhs->is_numeric() && rhs->is_numeric())                           \
            || (lhs->is<String>() && rhs->is<String>()))                       \
            return raw_##NAME(lhs->value_, rhs->value_);                       \
                                                                               \
        compare_err(#OP, lhs->type_name(), rhs->type_name());                  \
    }

DEF_ORDERING(less_than, <)
DEF_ORDERING(greater_than, >)
DEF_ORDERING(less_than_or_equal, <=)
DEF_ORDERING(greater_than_or_equal, >=)

} // namespace frst
