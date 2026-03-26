#include <frost/builtin.hpp>

using namespace frst;

Builtin::Builtin(function_t function, std::string name)
    : function_{std::move(function)}
    , name_{std::move(name)}
{
}

Value_Ptr Builtin::call(builtin_args_t args) const
{
    return function_(args);
}

std::string Builtin::debug_dump() const
{
    return fmt::format("<builtin:{}>", name_);
}

std::string Builtin::name() const
{
    return fmt::format("<builtin:{}>", name_);
}
