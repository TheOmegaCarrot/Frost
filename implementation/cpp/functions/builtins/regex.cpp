#include "builtins-common.hpp"

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <boost/regex.hpp>

#include <ranges>

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
        throw Frost_Recoverable_Error{fmt::format("Regex error: {}", e.what())};
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

namespace
{

std::optional<std::vector<std::string>> extract_group_names(
    const std::string& regex)
{
    std::vector<std::string> group_names;
    static const boost::regex re{R"(\(\?(<(?<aname>\w+)>|'(?<tname>\w+)'))",
                                 boost::regex_constants::optimize
                                     | boost::regex_constants::perl};
    for (boost::regex_iterator<std::string::const_iterator>
             it{regex.begin(), regex.end(), re},
         end;
         it != end; ++it)
    {
        const auto& match = *it;
        if (match["aname"].matched)
            group_names.push_back(match["aname"].str());
        if (match["tname"].matched)
            group_names.push_back(match["tname"].str());
    }

    if (not group_names.empty())
        return group_names;
    else
        return std::nullopt;
}

} // namespace

Value_Ptr scan_matches(builtin_args_t args)
{
    REQUIRE_ARGS("re.scan_matches", PARAM("string", TYPES(String)),
                 PARAM("regex", TYPES(String)));

    struct
    {
        Value_Ptr full = "full"_s;
        Value_Ptr matched = "matched"_s;
        Value_Ptr value = "value"_s;
        Value_Ptr index = "index"_s;
        Value_Ptr named = "named"_s;
        Value_Ptr groups = "groups"_s;
        Value_Ptr found = "found"_s;
        Value_Ptr count = "count"_s;
        Value_Ptr matches = "matches"_s;
    } static const keys;

    using itr = std::string::const_iterator;
    auto re = regex(args.at(1));
    const auto& input = args.at(0)->raw_get<String>();

    auto group_names = extract_group_names(args.at(1)->raw_get<String>());

    Array iterations{};
    Map result{};

    for (boost::regex_iterator<itr> it{input.begin(), input.end(), re}, end;
         it != end; ++it)
    {
        Map each_iteration{};
        Array groups{};
        const auto& matches = *it;
        groups.reserve(matches.size());
        for (const auto& [idx, match] : std::views::enumerate(matches))
        {
            auto value = [&] {
                if (match.matched)
                    return Value::create(match.str());
                else
                    return Value::null();
            }();
            if (idx == 0)
                each_iteration.try_emplace(keys.full, value);

            groups.push_back(Value::create(Map{
                {keys.matched, Value::create(auto{match.matched})},
                {keys.value, value},
                {keys.index, Value::create(auto{idx})},
            }));
        }

        if (group_names)
        {
            Map named{};
            for (const auto& group : group_names.value())
            {
                named.try_emplace(
                    Value::create(auto{group}),
                    Value::create(
                        Map{{keys.value,
                             [&] {
                                 if (matches[group].matched)
                                     return Value::create(matches[group].str());
                                 else
                                     return Value::null();
                             }()},
                            {keys.matched,
                             Value::create(auto{matches[group].matched})}}));
            }
            each_iteration.try_emplace(keys.named,
                                       Value::create(std::move(named)));
        }

        each_iteration.try_emplace(keys.groups,
                                   (Value::create(std::move(groups))));
        iterations.push_back(Value::create(std::move(each_iteration)));
    }

    result.try_emplace(keys.found, Value::create(not iterations.empty()));
    result.try_emplace(keys.count,
                       Value::create(static_cast<Int>(iterations.size())));
    result.try_emplace(keys.matches, Value::create(std::move(iterations)));

    return Value::create(std::move(result));
}

} // namespace re

void inject_regex(Symbol_Table& table)
{
    using namespace re;
    INJECT_MAP(re, ENTRY(matches, 2, 2), ENTRY(contains, 2, 2),
               ENTRY(replace, 3, 3), ENTRY(scan_matches, 2, 2));
}
} // namespace frst
