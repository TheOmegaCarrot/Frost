#ifndef FROST_OPERATOR_COMMON_HPP
#define FROST_OPERATOR_COMMON_HPP

#include <frost/value.hpp>

#include <fmt/format.h>

#include <string_view>

namespace frst
{
[[noreturn]] inline void op_err(std::string_view op_verb,
                                std::string_view op_glyph,
                                std::string_view lhs_type,
                                std::string_view rhs_type)
{
    throw Frost_Error{fmt::format("Cannot {} incompatible types: {} {} {}",
                                  op_verb, lhs_type, op_glyph, rhs_type)};
}

[[noreturn]] inline void compare_err(std::string_view op_glyph,
                                     std::string_view lhs_type,
                                     std::string_view rhs_type)
{
    op_err("compare", op_glyph, lhs_type, rhs_type);
}

} // namespace frst

#endif
