#include <frost/builtin.hpp>

using namespace frst;

Builtin::Builtin(function_t function, std::string name, const Arity& arity)
    : function_{std::move(function)}
    , name_{std::move(name)}
    , arity_{arity}
{
}

Value_Ptr Builtin::call(builtin_args_t args) const
{
    if (auto num_args = args.size(); arity_.max && num_args > arity_.max)
    {
        throw Frost_Recoverable_Error{
            fmt::format("Function {} called with too many arguments. "
                        "Called with {} but accepts no more than {}.",
                        name_, num_args, arity_.max.value())};
    }
    else if (num_args < arity_.min)
    {
        throw Frost_Recoverable_Error{
            fmt::format("Function {} called with insufficient arguments. "
                        "Called with {} but requires at least {}.",
                        name_, num_args, arity_.min)};
    }

    return function_(args);
}

std::string Builtin::debug_dump() const
{
    return fmt::format("<builtin:{}>", name_);
}
