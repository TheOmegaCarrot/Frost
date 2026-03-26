#include "frost/types.hpp"
#include <frost/builtins-common.hpp>

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <mutex>
#include <utility>

namespace frst
{

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
    REQUIRE_ARGS("mutable_cell", OPTIONAL(ANY));

    struct Wrapper
    {
        explicit Wrapper(Value_Ptr v)
            : value(std::move(v))
        {
        }
        Value_Ptr value;
        std::mutex mutex;
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
             system_closure([cell](builtin_args_t args) {
                 REQUIRE_ARGS("mutable_cell.exchange", ANY);
                 auto new_val = forbid_cycle(args.at(0));
                 std::lock_guard lock{cell->mutex};
                 return std::exchange(cell->value, std::move(new_val));
             })},
            {
                strings.get,
                system_closure([cell](builtin_args_t args) {
                    REQUIRE_NULLARY("mutable_cell.get");
                    std::lock_guard lock{cell->mutex};
                    return cell->value;
                }),
            },
        });
}

void inject_mutable_cell(Symbol_Table& table)
{
    INJECT(mutable_cell);
}
} // namespace frst
