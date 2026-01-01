#include <frost/value.hpp>

#include "operator-macros.hpp"

namespace frst
{

[[noreturn]] void add_err(std::string_view lhs_type, std::string_view rhs_type)
{
    op_err("add", lhs_type, rhs_type);
}

[[noreturn]] void sym_add_err(std::string_view type)
{
    add_err(type, type);
}

template <typename T>
[[noreturn]] void add_err_for()
{
    add_err(type_str<T>(), type_str<T>());
}

template <typename T>
[[noreturn]] void sym_add_err_for()
{
    sym_add_err(type_str<T>());
}

struct Add_Impl
{

    MATCH(Null, Null)
    {
        sym_add_err_for<Null>();
    }

    MATCH_LEFT(Null)
    {
        add_err_for<Null, RHS_T>();
    }

    MATCH_RIGHT(Null)
    {
        add_err_for<LHS_T, Null>();
    }

} add_impl;

Value_Ptr Value::add(const Value_Ptr& lhs, const Value_Ptr& rhs)
{
    const auto& lhs_var = lhs->value_;
    const auto& rhs_var = rhs->value_;

    return Value::create(std::visit(add_impl, lhs_var, rhs_var));
}

} // namespace frst
