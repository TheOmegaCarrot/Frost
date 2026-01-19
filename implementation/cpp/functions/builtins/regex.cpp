#include "builtins-common.hpp"

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <boost/regex.hpp>

namespace frst

{

namespace re
{

namespace
{
boost::regex regex(const Value_Ptr& re)
{
    try
    {
        return boost::regex{re->raw_get<String>()};
    }
    catch (const boost::regex_error& e)
    {
        throw Frost_User_Error{fmt::format("Regex error: {}", e.what())};
    }
}
} // namespace

Value_Ptr matches(builtin_args_t args)
{
    REQUIRE_ARGS("re.matches", PARAM("string", TYPES(String)),
                 PARAM("regex", TYPES(String)));

    auto re = regex(args.at(1));

    return Value::create(boost::regex_match(args.at(0)->raw_get<String>(), re));
}

Value_Ptr contains(builtin_args_t args)
{
    REQUIRE_ARGS("re.contains", PARAM("string", TYPES(String)),
                 PARAM("regex", TYPES(String)));

    auto re = regex(args.at(1));

    return Value::create(
        boost::regex_search(args.at(0)->raw_get<String>(), re));
}

Value_Ptr replace(builtin_args_t args)
{
    REQUIRE_ARGS("re.replace", PARAM("string", TYPES(String)),
                 PARAM("regex", TYPES(String)),
                 PARAM("replacement", TYPES(String)));

    auto re = regex(args.at(1));

    return Value::create(boost::regex_replace(args.at(0)->raw_get<String>(), re,
                                              args.at(2)->raw_get<String>()));
}

} // namespace re

void inject_regex(Symbol_Table& table)
{
    using namespace re;
    INJECT_MAP(re, ENTRY(matches, 2, 2), ENTRY(contains, 2, 2),
               ENTRY(replace, 3, 3));
}
} // namespace frst
