#include "builtins-common.hpp"

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <ranges>

namespace frst
{

Value_Ptr split(builtin_args_t args)
{
    REQUIRE_ARGS("split", TYPES(String), TYPES(String));

    const auto& target = args.at(0)->raw_get<String>();
    const auto& split_on = args.at(1)->raw_get<String>();

    if (split_on.size() != 1)
    {
        throw Frost_User_Error{
            fmt::format("Function split expected second argument to be length "
                        "1, but was length {}",
                        split_on.size())};
    }

    using std::views::transform, std::ranges::to;

    return Value::create(target
                         | std::views::split(split_on.front())
                         | transform(to<String>())
                         | transform([](String&& str) {
                               return Value::create(std::move(str));
                           })
                         | to<Array>());
}

#define X_BINARY_PASSTHROUGH                                                   \
    X(contains)                                                                \
    X(starts_with)                                                             \
    X(ends_with)

#define X(method)                                                              \
    Value_Ptr method(builtin_args_t args)                                      \
    {                                                                          \
        REQUIRE_ARGS(#method, TYPES(String), TYPES(String));                   \
                                                                               \
        return Value::create(args.at(0)->raw_get<String>().method(             \
            args.at(1)->raw_get<String>()));                                   \
    }

X_BINARY_PASSTHROUGH

#undef X

void inject_string_ops(Symbol_Table& table)
{
    INJECT(split, 2, 2);

#define X(fn) INJECT(fn, 2, 2);

    X_BINARY_PASSTHROUGH

#undef X
}

} // namespace frst
