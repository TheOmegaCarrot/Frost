#include <frost/extensions-common.hpp>

namespace frst
{
namespace unsafe
{

BUILTIN(mutable_cell)
{
    struct Wrapper
    {
        explicit Wrapper(Value_Ptr v)
            : value{std::move(v)}
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
            return args.at(0);
        }
    }());

    STRINGS(exchange, get);

    return Value::create(
        Value::trusted,
        Map{
            {strings.exchange,
             system_closure(1, 1,
                            [cell](builtin_args_t args) {
                                auto new_val = args.at(0);
                                std::lock_guard lock{cell->mutex};
                                return std::exchange(cell->value,
                                                     std::move(new_val));
                            })},
            {
                strings.get,
                system_closure(0, 0,
                               [cell](builtin_args_t) {
                                   std::lock_guard lock{cell->mutex};
                                   return cell->value;
                               }),
            },
        });
}

BUILTIN(weaken)
{
    struct Wrapper
    {
        explicit Wrapper(Value_Ptr v)
            : weak_value{v}
        {
        }

        std::weak_ptr<const Value> weak_value;
    };

    auto weak_ref = std::make_shared<Wrapper>(args.at(0));

    STRINGS(get);

    return Value::create(
        Value::trusted,
        Map{
            {strings.get,
             system_closure(0, 0,
                            [weak_ref](builtin_args_t) {
                                if (auto ptr = weak_ref->weak_value.lock())
                                    return ptr;
                                else
                                    return Value::null();
                            })},
        });
}

BUILTIN(identity)
{
    return Value::create(
        Int{reinterpret_cast<std::intptr_t>(args.at(0).get())});
}

BUILTIN(same)
{
    return Value::create(args.at(0).get() == args.at(1).get());
}

} // namespace unsafe

DECLARE_EXTENSION(unsafe)
{
    using namespace unsafe;
    CREATE_EXTENSION(ENTRY(identity, 1), ENTRY(same, 2),
                     ENTRY_R(mutable_cell, 0, 1), ENTRY(weaken, 1));
}
} // namespace frst
