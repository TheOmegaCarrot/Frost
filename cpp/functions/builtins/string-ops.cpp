#include <frost/builtins-common.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <boost/algorithm/string.hpp>
#include <ranges>

namespace frst
{
namespace
{
Value_Ptr do_split(const String& str, const String& delim)
{
    using std::views::transform, std::ranges::to;
    return Value::create(str
                         | std::views::split(delim)
                         | transform(to<String>())
                         | transform([](String&& str) {
                               return Value::create(std::move(str));
                           })
                         | to<Array>());
}
} // namespace

BUILTIN(split)
{
    REQUIRE_ARGS("split", TYPES(String), PARAM("delimiter", TYPES(String)));

    const auto& target = GET(0, String);
    const auto& split_on = GET(1, String);

    return do_split(target, split_on);
}

BUILTIN(lines)
{
    REQUIRE_ARGS("lines", TYPES(String));

    return do_split(GET(0, String), "\n");
}

BUILTIN(replace)
{
    REQUIRE_ARGS("replace", TYPES(String), PARAM("find", TYPES(String)),
                 PARAM("replacement", TYPES(String)));

    const auto& target = GET(0, String);
    const auto& find = GET(1, String);
    const auto& replace = GET(2, String);

    return Value::create(boost::replace_all_copy(target, find, replace));
}

BUILTIN(join)
{
    REQUIRE_ARGS("join", TYPES(Array), TYPES(String));

    const auto& target = GET(0, Array);
    const auto& joiner = GET(1, String);

    for (const auto& e : target)
    {
        if (not e->is<String>())
        {
            throw Frost_Recoverable_Error{fmt::format(
                "Function join requires Array of Strings as argument 1, got {}",
                e->type_name())};
        }
    }

    using std::views::transform, std::ranges::to;

    return Value::create(target
                         | transform([](const Value_Ptr& v) {
                               return v->raw_get<String>();
                           })
                         | std::views::join_with(joiner)
                         | to<String>());
}

#define X_UPPER_LOWER                                                          \
    X(to_upper)                                                                \
    X(to_lower)                                                                \
    X(trim_left)                                                               \
    X(trim_right)                                                              \
    X(trim)

#define X(ALG)                                                                 \
    BUILTIN(ALG)                                                               \
    {                                                                          \
        REQUIRE_ARGS(#ALG, TYPES(String));                                     \
                                                                               \
        return Value::create(boost::algorithm::ALG##_copy(GET(0, String)));    \
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
    INJECT(split);
    INJECT(lines);
    INJECT(replace);
    INJECT(join);
#define X(fn) INJECT(fn);

    X_BINARY_PASSTHROUGH

#undef X

#define X(ALG) INJECT(ALG);

    X_UPPER_LOWER

#undef X

}

} // namespace frst
