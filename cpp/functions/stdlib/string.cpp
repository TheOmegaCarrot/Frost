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

} // namespace string

STDLIB_MODULE(string, ENTRY(index_of), ENTRY(last_index_of), ENTRY(count),
              ENTRY(chars))

} // namespace frst
