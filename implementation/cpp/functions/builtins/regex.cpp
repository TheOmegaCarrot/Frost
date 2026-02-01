#include <frost/builtins-common.hpp>

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

boost::regex regex(const String& re)
{
    try
    {
        return boost::regex{re};
    }
    catch (const boost::regex_error& e)
    {
        throw Frost_Recoverable_Error{fmt::format("Regex error: {}", e.what())};
    }
}

} // namespace

BUILTIN(matches)
{
    REQUIRE_ARGS("re.matches", PARAM("string", TYPES(String)),
                 PARAM("regex", TYPES(String)));

    auto re = regex(GET(1, String));

    return Value::create(boost::regex_match(GET(0, String), re));
}

BUILTIN(contains)
{
    REQUIRE_ARGS("re.contains", PARAM("string", TYPES(String)),
                 PARAM("regex", TYPES(String)));

    auto re = regex(GET(1, String));

    return Value::create(boost::regex_search(GET(0, String), re));
}

BUILTIN(replace)
{
    REQUIRE_ARGS("re.replace", PARAM("string", TYPES(String)),
                 PARAM("regex", TYPES(String)),
                 PARAM("replacement", TYPES(String)));

    auto re = regex(GET(1, String));

    return Value::create(
        boost::regex_replace(GET(0, String), re, GET(2, String)));
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

BUILTIN(scan_matches)
{
    REQUIRE_ARGS("re.scan_matches", PARAM("string", TYPES(String)),
                 PARAM("regex", TYPES(String)));

    KEYS(full, matched, value, index, named, groups, found, count, matches);

    using itr = std::string::const_iterator;
    auto re = regex(GET(1, String));
    const auto& input = GET(0, String);

    auto group_names = extract_group_names(GET(1, String));

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
