#include "builtins-common.hpp"
#include "frost/builtin.hpp"

#include <fmt/core.h>

#include <frost/symbol-table.hpp>
#include <frost/utils.hpp>
#include <frost/value.hpp>

namespace frst
{

namespace
{

std::string mformat_impl(const String& fmt_str, const Map& repl_map)
{
    auto segments_e = utils::parse_fmt_string(fmt_str);

    if (not segments_e.has_value())
        throw Frost_Recoverable_Error{segments_e.error()};

    auto& segments = segments_e.value();

    std::string out;
    out.reserve(fmt_str.size());

    for (const auto& segment : segments)
    {
        segment.visit(
            Overload{[&](const utils::Fmt_Literal& lit) {
                         out.append(lit.text);
                     },
                     [&](const utils::Fmt_Placeholder& placeholder) {
                         auto key = Value::create(String{placeholder.text});
                         auto map_itr = repl_map.find(key);
                         if (map_itr == repl_map.end())
                             throw Frost_Recoverable_Error{
                                 fmt::format("Missing replacement for key: {}",
                                             key->raw_get<String>())};

                         out.append(map_itr->second->to_internal_string());
                     }});
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
