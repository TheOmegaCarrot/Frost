#include <frost/extensions-common.hpp>

namespace frst
{
namespace unsafe
{

BUILTIN(mutable_cell)
{
    REQUIRE_ARGS("unsafe.mutable_cell", OPTIONAL(ANY));

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
            return args.at(0);
    }());

    STRINGS(exchange, get);

    return Value::create(
        Value::trusted,
        Map{
            {strings.exchange, system_closure([cell](builtin_args_t args) {
                 REQUIRE_ARGS("unsafe.mutable_cell.exchange", ANY);
                 auto new_val = args.at(0);
                 std::lock_guard lock{cell->mutex};
                 return std::exchange(cell->value, std::move(new_val));
             })},
            {
                strings.get,
                system_closure([cell](builtin_args_t args) {
                    REQUIRE_NULLARY("unsafe.mutable_cell.get");
                    std::lock_guard lock{cell->mutex};
                    return cell->value;
                }),
            },
        });
}

BUILTIN(weaken)
{
    REQUIRE_ARGS("unsafe.weaken", ANY);

    STRINGS(get);

    return Value::create(
        Value::trusted,
        Map{
            {strings.get,
             system_closure([weak_ref = std::weak_ptr<const Value>(args.at(0))](
                                builtin_args_t args) {
                 REQUIRE_NULLARY("unsafe.weaken.get");
                 if (auto ptr = weak_ref.lock())
                     return ptr;
                 else
                     return Value::null();
             })},
        });
}

BUILTIN(identity)
{
    REQUIRE_ARGS("unsafe.identity", ANY);
    return Value::create(
        Int{reinterpret_cast<std::intptr_t>(args.at(0).get())});
}

BUILTIN(same)
{
    REQUIRE_ARGS("unsafe.same", ANY, ANY);
    return Value::create(args.at(0).get() == args.at(1).get());
}

} // namespace unsafe

REGISTER_EXTENSION(unsafe, ENTRY(identity), ENTRY(same), ENTRY(mutable_cell),
                   ENTRY(weaken));
} // namespace frst
