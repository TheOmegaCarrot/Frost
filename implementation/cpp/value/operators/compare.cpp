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

} // namespace frst
