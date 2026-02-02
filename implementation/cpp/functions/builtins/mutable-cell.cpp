#include <frost/builtins-common.hpp>

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

Value_Ptr forbid_cycle(Value_Ptr value)
{
    if (!value->is_primitive())
        throw Frost_Recoverable_Error{
            "Non-primitive values may not be stored in a mutable_cell"};
    return value;
}

BUILTIN(mutable_cell)
{
    struct Wrapper
    {
        Value_Ptr value;
    };

    auto cell = std::make_shared<Wrapper>([&] {
        if (args.empty())
            return Value::null();
        else
        {
            return forbid_cycle(args.at(0));
        }
    }());

    STRINGS(exchange, get);

    return Value::create(Map{
        {strings.exchange, system_closure(1, 1,
                                          [cell](builtin_args_t args) mutable {
                                              return std::exchange(
                                                  cell->value,
                                                  forbid_cycle(args.at(0)));
                                          })},
        {
            strings.get,
            system_closure(0, 0,
                           [cell](builtin_args_t) {
                               return cell->value;
                           }),
        },
    });
}

void inject_mutable_cell(Symbol_Table& table)
{
    INJECT(mutable_cell, 0, 1);
}
} // namespace frst
