#include "builtins-common.hpp"

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <utility>

namespace frst
{

struct Mutable_Cell
{
    Value_Ptr value;

    Value_Ptr get() const
    {
        return value->clone();
    }

    void set(Value_Ptr new_value)
    {
        value = new_value;
    }
};

Value_Ptr mutable_cell(builtin_args_t args)
{
    struct Wrapper
    {
        Value_Ptr value;
    };

    auto cell = std::make_shared<Wrapper>([&] {
        if (args.empty())
            return Value::null();
        else
            return args.front();
    }());

    return Value::create(Map{
        {"exchange"_s,
         Value::create(Function{std::make_shared<Builtin>(
             [cell](builtin_args_t args) mutable {
                 return std::exchange(cell->value, args.at(0));
             },
             "mutable cell exchange", Builtin::Arity{.min = 1, .max = 1})})},
        {"get"_s,
         Value::create(Function{std::make_shared<Builtin>(
             [cell](builtin_args_t) {
                 return cell->value->clone();
             },
             "mutable cell getter", Builtin::Arity{.min = 0, .max = 0})})}});
}

void inject_mutable_cell(Symbol_Table& table)
{
    INJECT(mutable_cell, 0, 1);
}
} // namespace frst
