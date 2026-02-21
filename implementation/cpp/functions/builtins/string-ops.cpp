#include <frost/builtins-common.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <boost/algorithm/string/case_conv.hpp>
#include <cppcodec/base64_rfc4648.hpp>
#include <cppcodec/base64_url.hpp>

#include <ranges>

namespace frst
{

BUILTIN(split)
{
    REQUIRE_ARGS("split", TYPES(String), TYPES(String));

    const auto& target = GET(0, String);
    const auto& split_on = GET(1, String);

    using std::views::transform, std::ranges::to;

    return Value::create(target
                         | std::views::split(split_on)
                         | transform(to<String>())
                         | transform([](String&& str) {
                               return Value::create(std::move(str));
                           })
                         | to<Array>());
}

BUILTIN(b64_encode)
{
    REQUIRE_ARGS("b64_encode", TYPES(String));

    try
    {
        return Value::create(cppcodec::base64_rfc4648::encode(GET(0, String)));
    }
    catch (const std::exception& e)
    {
        throw Frost_Recoverable_Error{String{e.what()}};
    }
}

BUILTIN(b64_decode)
{
    REQUIRE_ARGS("b64_decode", TYPES(String));

    try
    {
        return Value::create(String(
            std::from_range, cppcodec::base64_rfc4648::decode(GET(0, String))));
    }
    catch (const std::exception& e)
    {
        throw Frost_Recoverable_Error{String{e.what()}};
    }
}

BUILTIN(b64_urlencode)
{
    REQUIRE_ARGS("b64_urlencode", TYPES(String));

    try
    {
        return Value::create(cppcodec::base64_url::encode(GET(0, String)));
    }
    catch (const std::exception& e)
    {
        throw Frost_Recoverable_Error{String{e.what()}};
    }
}

BUILTIN(b64_urldecode)
{
    REQUIRE_ARGS("b64_urldecode", TYPES(String));

    try
    {
        return Value::create(String(
            std::from_range, cppcodec::base64_url::decode(GET(0, String))));
    }
    catch (const std::exception& e)
    {
        throw Frost_Recoverable_Error{String{e.what()}};
    }
}

BUILTIN(fmt_int)
{
    REQUIRE_ARGS("fmt_num", PARAM("number", TYPES(Int)),
                 PARAM("base", TYPES(Int)));

    const Int input = GET(0, Int);
    const Int base = GET(1, Int);

    if (base < 2 || base > 36)
    {
        throw Frost_Recoverable_Error{fmt::format(
            "fmt_int given base of {}, but base must be in range [2, 36]",
            base)};
    }

    char buf[66]{};

    const auto [ptr, ec] =
        std::to_chars(std::begin(buf), std::end(buf), input, base);

    // This really shouldn't happen, the buffer is big enough for any input
    if (ec != std::errc{})
        THROW_UNREACHABLE;

    return Value::create(String{std::begin(buf), ptr});
}

BUILTIN(parse_int)
{
    REQUIRE_ARGS("parse_int", PARAM("number", TYPES(String)),
                 PARAM("base", TYPES(Int)));

    const String& input = GET(0, String);
    const Int base = GET(1, Int);

    if (base < 2 || base > 36)
    {
        throw Frost_Recoverable_Error{fmt::format(
            "parse_int given base of {}, but base must be in range [2, 36]",
            base)};
    }

    Int result{};
    const auto [ptr, ec] = std::from_chars(
        input.data(), input.data() + input.size(), result, base);

    if ((ec == std::errc::invalid_argument)
        || (ptr != input.data() + input.size()))
    {
        throw Frost_Recoverable_Error{fmt::format(
            "parse_int expected numeric string in base {}, but got \"{}\"",
            base, input)};
    }

    if (ec == std::errc::result_out_of_range)
    {
        throw Frost_Recoverable_Error{fmt::format(
            "parse_int cannot parse \"{}\", which is out of range", input)};
    }

    return Value::create(result);
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
    INJECT(b64_encode, 1, 1);
    INJECT(b64_decode, 1, 1);
    INJECT(b64_urlencode, 1, 1);
    INJECT(b64_urldecode, 1, 1);

#define X(fn) INJECT(fn, 2, 2);

    X_BINARY_PASSTHROUGH

#undef X

#define X(case) INJECT(to_##case, 1, 1);

    X_UPPER_LOWER

#undef X

    INJECT(fmt_int, 2, 2);
    INJECT(parse_int, 2, 2);
}

} // namespace frst
