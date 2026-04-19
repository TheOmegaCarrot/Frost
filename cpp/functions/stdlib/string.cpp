#include <frost/builtins-common.hpp>
#include <frost/value.hpp>

namespace frst
{

namespace string
{

// -- Substring operations --

BUILTIN(index_of)
{
    REQUIRE_ARGS("string.index_of", PARAM("s", TYPES(String)),
                 PARAM("substr", TYPES(String)));

    auto pos = GET(0, String).find(GET(1, String));
    if (pos == std::string::npos)
        return Value::null();
    return Value::create(Int{static_cast<Int>(pos)});
}

BUILTIN(last_index_of)
{
    REQUIRE_ARGS("string.last_index_of", PARAM("s", TYPES(String)),
                 PARAM("substr", TYPES(String)));

    auto pos = GET(0, String).rfind(GET(1, String));
    if (pos == std::string::npos)
        return Value::null();
    return Value::create(Int{static_cast<Int>(pos)});
}

BUILTIN(count)
{
    REQUIRE_ARGS("string.count", PARAM("s", TYPES(String)),
                 PARAM("substr", TYPES(String)));

    const auto& s = GET(0, String);
    const auto& substr = GET(1, String);

    if (substr.empty())
        throw Frost_Recoverable_Error{
            "string.count: substring must not be empty"};

    Int n = 0;
    std::string::size_type pos = 0;
    while ((pos = s.find(substr, pos)) != std::string::npos)
    {
        ++n;
        pos += substr.size();
    }
    return Value::create(n);
}

BUILTIN(chars)
{
    REQUIRE_ARGS("string.chars", PARAM("s", TYPES(String)));

    const auto& s = GET(0, String);
    return Value::create(s
                         | std::views::transform([](char c) {
                               return Value::create(String{c});
                           })
                         | std::ranges::to<Array>());
}

// -- Character classification --

BUILTIN(is_empty)
{
    REQUIRE_ARGS("string.is_empty", PARAM("s", TYPES(String)));
    return Value::create(Bool{GET(0, String).empty()});
}

BUILTIN(is_ascii)
{
    REQUIRE_ARGS("string.is_ascii", PARAM("s", TYPES(String)));
    return Value::create(Bool{std::ranges::all_of(
        GET(0, String),
        [](unsigned char c) { return c < 128; })});
}

BUILTIN(is_digit)
{
    REQUIRE_ARGS("string.is_digit", PARAM("s", TYPES(String)));
    return Value::create(Bool{std::ranges::all_of(
        GET(0, String),
        [](unsigned char c) { return std::isdigit(c); })});
}

BUILTIN(is_alpha)
{
    REQUIRE_ARGS("string.is_alpha", PARAM("s", TYPES(String)));
    return Value::create(Bool{std::ranges::all_of(
        GET(0, String),
        [](unsigned char c) { return std::isalpha(c); })});
}

BUILTIN(is_alphanumeric)
{
    REQUIRE_ARGS("string.is_alphanumeric", PARAM("s", TYPES(String)));
    return Value::create(Bool{std::ranges::all_of(
        GET(0, String),
        [](unsigned char c) { return std::isalnum(c); })});
}

BUILTIN(is_whitespace)
{
    REQUIRE_ARGS("string.is_whitespace", PARAM("s", TYPES(String)));
    return Value::create(Bool{std::ranges::all_of(
        GET(0, String),
        [](unsigned char c) { return std::isspace(c); })});
}

BUILTIN(is_uppercase)
{
    REQUIRE_ARGS("string.is_uppercase", PARAM("s", TYPES(String)));
    return Value::create(Bool{std::ranges::all_of(
        GET(0, String),
        [](unsigned char c) { return not std::isalpha(c) || std::isupper(c); })});
}

BUILTIN(is_lowercase)
{
    REQUIRE_ARGS("string.is_lowercase", PARAM("s", TYPES(String)));
    return Value::create(Bool{std::ranges::all_of(
        GET(0, String),
        [](unsigned char c) { return not std::isalpha(c) || std::islower(c); })});
}

// -- Padding --

BUILTIN(pad_left)
{
    REQUIRE_ARGS("string.pad_left", PARAM("s", TYPES(String)),
                 PARAM("width", TYPES(Int)),
                 OPTIONAL(PARAM("fill", TYPES(String))));

    const auto& s = GET(0, String);
    auto width = GET(1, Int);

    char fill = ' ';
    if (HAS(2))
    {
        const auto& fill_str = GET(2, String);
        if (fill_str.size() != 1)
            throw Frost_Recoverable_Error{
                "string.pad_left: fill must be a single byte"};
        fill = fill_str[0];
    }

    if (static_cast<Int>(s.size()) >= width)
        return Value::create(String{s});

    return Value::create(
        String(static_cast<std::size_t>(width) - s.size(), fill) + s);
}

BUILTIN(pad_right)
{
    REQUIRE_ARGS("string.pad_right", PARAM("s", TYPES(String)),
                 PARAM("width", TYPES(Int)),
                 OPTIONAL(PARAM("fill", TYPES(String))));

    const auto& s = GET(0, String);
    auto width = GET(1, Int);

    char fill = ' ';
    if (HAS(2))
    {
        const auto& fill_str = GET(2, String);
        if (fill_str.size() != 1)
            throw Frost_Recoverable_Error{
                "string.pad_right: fill must be a single byte"};
        fill = fill_str[0];
    }

    if (static_cast<Int>(s.size()) >= width)
        return Value::create(String{s});

    return Value::create(
        s + String(static_cast<std::size_t>(width) - s.size(), fill));
}

BUILTIN(center)
{
    REQUIRE_ARGS("string.center", PARAM("s", TYPES(String)),
                 PARAM("width", TYPES(Int)),
                 OPTIONAL(PARAM("fill", TYPES(String))));

    const auto& s = GET(0, String);
    auto width = GET(1, Int);

    char fill = ' ';
    if (HAS(2))
    {
        const auto& fill_str = GET(2, String);
        if (fill_str.size() != 1)
            throw Frost_Recoverable_Error{
                "string.center: fill must be a single byte"};
        fill = fill_str[0];
    }

    if (static_cast<Int>(s.size()) >= width)
        return Value::create(String{s});

    auto total_pad = static_cast<std::size_t>(width) - s.size();
    auto left_pad = total_pad / 2;
    auto right_pad = total_pad - left_pad;

    return Value::create(String(left_pad, fill) + s + String(right_pad, fill));
}

// -- Transforms --

BUILTIN(repeat)
{
    REQUIRE_ARGS("string.repeat", PARAM("s", TYPES(String)),
                 PARAM("n", TYPES(Int)));

    const auto& s = GET(0, String);
    auto n = GET(1, Int);

    if (n < 0)
        throw Frost_Recoverable_Error{
            "string.repeat: count must not be negative"};

    String result;
    result.reserve(s.size() * static_cast<std::size_t>(n));
    for (Int i = 0; i < n; ++i)
        result += s;
    return Value::create(std::move(result));
}

} // namespace string

STDLIB_MODULE(string, ENTRY(index_of), ENTRY(last_index_of), ENTRY(count),
              ENTRY(chars), ENTRY(is_empty), ENTRY(is_ascii), ENTRY(is_digit),
              ENTRY(is_alpha), ENTRY(is_alphanumeric), ENTRY(is_whitespace),
              ENTRY(is_uppercase), ENTRY(is_lowercase), ENTRY(pad_left),
              ENTRY(pad_right), ENTRY(center), ENTRY(repeat))

} // namespace frst
