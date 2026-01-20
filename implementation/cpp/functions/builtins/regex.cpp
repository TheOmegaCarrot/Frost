#include "builtins-common.hpp"

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <boost/regex.hpp>

using namespace std::literals;
using namespace frst::literals;

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

Value_Ptr decompose(builtin_args_t args)
{
    REQUIRE_ARGS("re.decompose", PARAM("string", TYPES(String)),
                 PARAM("regex", TYPES(String)),
                 OPTIONAL(PARAM("groups", TYPES(Array))));

    bool use_groups = args.size() == 3;
    if (use_groups)
        for (const auto& elem : args.at(2)->raw_get<Array>())
        {
            if (not elem->is<String>())
                throw Frost_User_Error{
                    fmt::format("Group names must be String, but got {}",
                                elem->type_name())};
        }

    using itr = std::string::const_iterator;
    auto re = regex(args.at(1));
    const auto& input = args.at(0)->raw_get<String>();

    Array iterations{};
    Map result{};

    for (boost::regex_iterator<itr> it{input.begin(), input.end(), re}, end;
         it != end; ++it)
    {
        Map each_iteration{};
        Array each_match{};
        const auto& matches = *it;
        for (const auto& match : matches)
        {
            each_match.push_back(Value::create(Map{
                {"matched"_s, Value::create(auto{match.matched})},
                {"value"_s,
                 [&] {
                     if (match.matched)
                         return Value::create(match.str());
                     else
                         return Value::null();
                 }()},
            }));
        }

        if (use_groups)
        {
            Map named_groups{};
            for (const auto& elem : args.at(2)->raw_get<Array>())
            {
                const auto& group = elem->raw_get<String>();
                named_groups.insert_or_assign(elem, [&] {
                    if (matches[group].matched)
                        return Value::create(matches[group].str());
                    else
                        return Value::null();
                }());
            }
            each_iteration.insert_or_assign(
                "named"_s, Value::create(std::move(named_groups)));
        }

        each_iteration.insert_or_assign("groups"_s,
                                        (Value::create(std::move(each_match))));
        iterations.push_back(Value::create(std::move(each_iteration)));
    }

    result.insert_or_assign("found"_s, Value::create(not iterations.empty()));
    result.insert_or_assign("count"_s,
                            Value::create(static_cast<Int>(iterations.size())));
    result.insert_or_assign("matches"_s, Value::create(std::move(iterations)));

    return Value::create(std::move(result));
}

} // namespace re

void inject_regex(Symbol_Table& table)
{
    using namespace re;
    INJECT_MAP(re, ENTRY(matches, 2, 2), ENTRY(contains, 2, 2),
               ENTRY(replace, 3, 3), ENTRY(decompose, 2, 3));
}
} // namespace frst
