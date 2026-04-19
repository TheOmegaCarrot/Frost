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

} // namespace string

STDLIB_MODULE(string, ENTRY(index_of), ENTRY(last_index_of), ENTRY(count),
              ENTRY(chars), ENTRY(is_empty), ENTRY(is_ascii), ENTRY(is_digit),
              ENTRY(is_alpha), ENTRY(is_alphanumeric), ENTRY(is_whitespace),
              ENTRY(is_uppercase), ENTRY(is_lowercase))

} // namespace frst
