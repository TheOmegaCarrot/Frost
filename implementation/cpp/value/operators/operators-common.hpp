#ifndef FROST_OPERATOR_COMMON_HPP
#define FROST_OPERATOR_COMMON_HPP

#include <frost/value.hpp>

#include <format>
#include <string_view>

namespace frst
{
[[noreturn]] inline void op_err(std::string_view op_verb,
                                std::string_view lhs_type,
                                std::string_view rhs_type)
{
    throw Frost_Error{std::format("Cannot {} incompatible types: {} and {}",
                                  op_verb, lhs_type, rhs_type)};
}

template <typename T>
concept Frost_Numeric = std::same_as<Int, T> || std::same_as<Float, T>;

} // namespace frst

#endif
