#include "frost/types.hpp"
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
        return value;
    }

    void set(Value_Ptr new_value)
    {
        value = new_value;
    }
};

struct
{

    void operator()(this const auto, const Frost_Primitive auto&)
    {
    }

    void operator()(this const auto recurse, const Array& arr)
    {
        for (const auto& elem : arr)
            elem->visit(recurse);
    }

    void operator()(this const auto recurse, const Map& map)
    {
        for (const auto& [k, v] : map)
        {
            k->visit(recurse);
            v->visit(recurse);
        }
    }

    void operator()(this const auto, const Function&)
    {
        throw Frost_Recoverable_Error{
            "Function values may not be stored in a mutable_cell. "
            "This includes functions nested inside of structures."};
    }

} constexpr static forbid_cycle_fn;

Value_Ptr forbid_cycle(Value_Ptr value)
{
    value->visit(forbid_cycle_fn);
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

    return Value::create(
        Value::trusted,
        Map{
            {strings.exchange,
             system_closure(1, 1,
                            [cell](builtin_args_t args) mutable {
                                return std::exchange(cell->value,
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
