#include <frost/builtins-common.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <boost/algorithm/string/case_conv.hpp>

#include <ranges>

namespace frst
{

BUILTIN(split)
{
    REQUIRE_ARGS("split", TYPES(String), TYPES(String));

    const auto& target = GET(0, String);
    const auto& split_on = GET(1, String);

    if (split_on.size() != 1)
    {
        throw Frost_Recoverable_Error{
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

#define X_UPPER_LOWER                                                          \
    X(upper)                                                                   \
    X(lower)

#define X(case)                                                                \
    BUILTIN(to_##case)                                                         \
    {                                                                          \
        REQUIRE_ARGS("to_" #case, TYPES(String));                              \
                                                                               \
        return Value::create(                                                  \
            boost::algorithm::to_##case##_copy(GET(0, String)));               \
    }

X_UPPER_LOWER

#undef X

#define X_BINARY_PASSTHROUGH                                                   \
    X(contains)                                                                \
    X(starts_with)                                                             \
    X(ends_with)

#define X(method)                                                              \
    BUILTIN(method)                                                            \
    {                                                                          \
        REQUIRE_ARGS(#method, TYPES(String), TYPES(String));                   \
                                                                               \
        return Value::create(GET(0, String).method(GET(1, String)));           \
    }

X_BINARY_PASSTHROUGH

#undef X

void inject_string_ops(Symbol_Table& table)
{
    INJECT(split, 2, 2);

#define X(fn) INJECT(fn, 2, 2);

    X_BINARY_PASSTHROUGH

#undef X

#define X(case) INJECT(to_##case, 1, 1);

    X_UPPER_LOWER

#undef X
}

} // namespace frst
