#ifndef FROST_OPERATOR_MACROS_HPP
#define FROST_OPERATOR_MACROS_HPP

#include <frost/value.hpp>

#include <format>
#include <string_view>

#define MATCH(LHS_T, RHS_T)                                                    \
    static Value operator()(const LHS_T& lhs, const RHS_T& rhs)

#define MATCH_LEFT(LHS_T)                                                      \
    template <Frost_Type RHS_T>                                                \
    static Value operator()(const LHS_T& lhs, const RHS_T& rhs)

#define MATCH_RIGHT(RHS_T)                                                     \
    template <Frost_Type LHS_T>                                                \
    static Value operator()(const LHS_T& lhs, const RHS_T& rhs)

namespace frst
{
[[noreturn]] inline void op_err(std::string_view op_verb,
                                std::string_view lhs_type,
                                std::string_view rhs_type)
{
    throw Frost_Error{std::format("Cannot {} incompatible types: {} and {}",
                                  op_verb, lhs_type, rhs_type)};
}
} // namespace frst

#endif
