#include "builtins-common.hpp"
#include "frost/builtin.hpp"

#include <fmt/core.h>

#include <boost/regex.hpp>

#include <frost/symbol-table.hpp>
#include <frost/utils.hpp>
#include <frost/value.hpp>

#include <ranges>

namespace frst
{

namespace
{

std::string mformat_impl(const String& fmt_str, const Map& repl_map)
{
    auto replacements_e = utils::parse_fmt_string(fmt_str);

    if (not replacements_e.has_value())
        throw Frost_Recoverable_Error{replacements_e.error()};

    auto& replacements = replacements_e.value();

    static const auto re = boost::regex(R"(\\)");
    std::string out = boost::regex_replace(fmt_str, re, "\\");
    for (auto& replacement : std::views::reverse(replacements))
    {
        auto key = Value::create(std::move(replacement.content));
        auto map_itr = repl_map.find(key);
        if (map_itr == repl_map.end())
            throw Frost_Recoverable_Error{fmt::format(
                "Missing replacement for key: {}", key->raw_get<String>())};

        auto formatted_replacment = map_itr->second->to_internal_string();

        out.replace(replacement.start, replacement.len, formatted_replacment);
    }

    return out;
}

} // namespace

BUILTIN(mformat)
{
    REQUIRE_ARGS("mformat", PARAM("format string", TYPES(String)),
                 PARAM("replacement map", TYPES(Map)));

    return Value::create(mformat_impl(GET(0, String), GET(1, Map)));
}

BUILTIN(mprint)
{
    REQUIRE_ARGS("mprint", PARAM("format string", TYPES(String)),
                 PARAM("replacement map", TYPES(Map)));

    std::puts(mformat_impl(GET(0, String), GET(1, Map)).c_str());

    return Value::null();
}

BUILTIN(print)
{
    std::puts(args.at(0)->to_internal_string().c_str());

    return Value::null();
}

void inject_output(Symbol_Table& table)
{
    INJECT(mformat, 2, 2);
    INJECT(mprint, 2, 2);
    INJECT(print, 1, 1);
}
} // namespace frst
