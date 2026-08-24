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

namespace regex
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
    REQUIRE_ARGS("regex.matches", PARAM("string", TYPES(String)),
                 PARAM("regex", TYPES(String)));

    auto re = regex(GET(1, String));

    return Value::create(boost::regex_match(GET(0, String), re));
}

BUILTIN(contains)
{
    REQUIRE_ARGS("regex.contains", PARAM("string", TYPES(String)),
                 PARAM("regex", TYPES(String)));

    auto re = regex(GET(1, String));

    return Value::create(boost::regex_search(GET(0, String), re));
}

BUILTIN(replace)
{
    REQUIRE_ARGS("regex.replace", PARAM("string", TYPES(String)),
                 PARAM("regex", TYPES(String)),
                 PARAM("replacement", TYPES(String)));

    auto re = regex(GET(1, String));

    return Value::create(
        boost::regex_replace(GET(0, String), re, GET(2, String)));
}

BUILTIN(replace_first)
{
    REQUIRE_ARGS("regex.replace_first", PARAM("string", TYPES(String)),
                 PARAM("regex", TYPES(String)),
                 PARAM("replacement", TYPES(String)));

    auto re = regex(GET(1, String));

    return Value::create(
        boost::regex_replace(GET(0, String), re, GET(2, String),
                             boost::regex_constants::format_first_only));
}

BUILTIN(replace_with)
{
    REQUIRE_ARGS("regex.replace_with", PARAM("string", TYPES(String)),
                 PARAM("regex", TYPES(String)),
                 PARAM("callback", TYPES(Function)));

    auto re = regex(GET(1, String));
    const auto& input = GET(0, String);
    const auto& fn = GET(2, Function);

    using itr = boost::regex_iterator<String::const_iterator>;

    std::string result;
    auto tail = input.cbegin();

    for (itr it{input.cbegin(), input.cend(), re}, end; it != end; ++it)
    {
        const auto& match = *it;
        result.append(tail, match[0].first);
        auto replacement = fn->call({Value::create(match.str())});
        result += replacement->to_internal_string();
        tail = match[0].second;
    }

    result.append(tail, input.cend());
    return Value::create(std::move(result));
}

BUILTIN(split)
{
    REQUIRE_ARGS("regex.split", PARAM("string", TYPES(String)),
                 PARAM("regex", TYPES(String)));

    auto re = regex(GET(1, String));
    const auto& input = GET(0, String);

    using itr = boost::regex_token_iterator<String::const_iterator>;
    return Value::create(
        std::ranges::subrange(itr{input.begin(), input.end(), re, -1}, itr{})
        | std::views::transform([](const auto& m) {
              return Value::create(String{m.first, m.second});
          })
        | std::ranges::to<Array>());
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
    REQUIRE_ARGS("regex.scan_matches", PARAM("string", TYPES(String)),
                 PARAM("regex", TYPES(String)));

    STRINGS(full, matched, value, index, named, groups, found, count, matches);

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
                each_iteration.try_emplace(strings.full, value);

            groups.push_back(Value::create(Map{
                {strings.matched, Value::create(auto{match.matched})},
                {strings.value, value},
                {strings.index, Value::create(auto{idx})},
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
                        Value::trusted,
                        Map{{strings.value,
                             [&] {
                                 if (matches[group].matched)
                                     return Value::create(matches[group].str());
                                 else
                                     return Value::null();
                             }()},
                            {strings.matched,
                             Value::create(auto{matches[group].matched})}}));
            }
            each_iteration.try_emplace(
                strings.named, Value::create(Value::trusted, std::move(named)));
        }

        each_iteration.try_emplace(strings.groups,
                                   (Value::create(std::move(groups))));
        iterations.push_back(
            Value::create(Value::trusted, std::move(each_iteration)));
    }

    result.try_emplace(strings.found, Value::create(not iterations.empty()));
    result.try_emplace(strings.count,
                       Value::create(static_cast<Int>(iterations.size())));
    result.try_emplace(strings.matches, Value::create(std::move(iterations)));

    return Value::create(Value::trusted, std::move(result));
}

} // namespace regex

STDLIB_MODULE(regex, ENTRY(matches), ENTRY(contains), ENTRY(replace),
              ENTRY(replace_first), ENTRY(replace_with), ENTRY(split),
              ENTRY(scan_matches))
} // namespace frst
