#include <frost/builtins-common.hpp>
#include <frost/value.hpp>

#include <chrono>
#include <sstream>

namespace frst
{

namespace datetime
{

namespace
{

using Sys_Millis = std::chrono::time_point<std::chrono::system_clock,
                                           std::chrono::milliseconds>;

Sys_Millis millis_to_tp(Int ms)
{
    return Sys_Millis{std::chrono::milliseconds{ms}};
}

Int tp_to_millis(Sys_Millis tp)
{
    return tp.time_since_epoch().count();
}

STRINGS(year, month, day, hour, minute, second, ms, week, weekday, Sunday,
        Monday, Tuesday, Wednesday, Thursday, Friday, Saturday);

constexpr auto ms_per(auto dur)
{
    return std::chrono::duration_cast<std::chrono::milliseconds>(dur).count();
}

constexpr std::array weekday_names{
    &strings.Sunday,   &strings.Monday, &strings.Tuesday,  &strings.Wednesday,
    &strings.Thursday, &strings.Friday, &strings.Saturday,
};

const std::chrono::time_zone* locate_tz(const String& name)
{
    try
    {
        return std::chrono::locate_zone(name);
    }
    catch (const std::runtime_error&)
    {
        throw Frost_Recoverable_Error{
            fmt::format("datetime: unknown timezone '{}'", name)};
    }
}

} // namespace

BUILTIN(now)
{
    REQUIRE_NULLARY("datetime.now");

    auto tp = std::chrono::time_point_cast<std::chrono::milliseconds>(
        std::chrono::system_clock::now());
    return Value::create(tp_to_millis(tp));
}

BUILTIN(format)
{
    REQUIRE_ARGS("datetime.format", PARAM("millis", TYPES(Int)),
                 PARAM("pattern", TYPES(String)),
                 OPTIONAL(PARAM("timezone", TYPES(String))));

    auto tp = millis_to_tp(GET(0, Int));
    const auto& pattern = GET(1, String);
    auto fmt_str = "{:" + pattern + "}";

    try
    {
        if (HAS(2))
        {
            auto zt = std::chrono::zoned_time{
                locate_tz(GET(2, String)), tp};
            return Value::create(String{
                std::vformat(fmt_str, std::make_format_args(zt))});
        }
        return Value::create(String{
            std::vformat(fmt_str, std::make_format_args(tp))});
    }
    catch (const std::format_error& e)
    {
        throw Frost_Recoverable_Error{
            fmt::format("datetime.format: {}", e.what())};
    }
}

BUILTIN(parse)
{
    REQUIRE_ARGS("datetime.parse", PARAM("s", TYPES(String)),
                 PARAM("pattern", TYPES(String)),
                 OPTIONAL(PARAM("timezone", TYPES(String))));

    const auto& input = GET(0, String);
    const auto& pattern = GET(1, String);

    if (HAS(2))
    {
        const auto* tz = locate_tz(GET(2, String));
        std::chrono::local_time<std::chrono::milliseconds> local_tp{};
        std::istringstream iss{input};
        iss >> std::chrono::parse(pattern, local_tp);
        if (iss.fail())
            throw Frost_Recoverable_Error{fmt::format(
                "datetime.parse: '{}' does not match pattern '{}'", input,
                pattern)};

        auto zt = std::chrono::zoned_time{tz, local_tp};
        return Value::create(tp_to_millis(
            std::chrono::time_point_cast<std::chrono::milliseconds>(
                zt.get_sys_time())));
    }

    Sys_Millis tp{};
    std::istringstream iss{input};
    iss >> std::chrono::parse(pattern, tp);
    if (iss.fail())
        throw Frost_Recoverable_Error{
            fmt::format("datetime.parse: '{}' does not match pattern '{}'",
                        input, pattern)};

    return Value::create(tp_to_millis(tp));
}

BUILTIN(components)
{
    REQUIRE_ARGS("datetime.components", PARAM("millis", TYPES(Int)),
                 OPTIONAL(PARAM("timezone", TYPES(String))));

    auto tp = millis_to_tp(GET(0, Int));

    // Get the time point to decompose — either UTC or local
    auto decompose = [](auto time_point) -> Value_Ptr {
        auto dp = std::chrono::floor<std::chrono::days>(time_point);
        std::chrono::year_month_day ymd{dp};
        std::chrono::hh_mm_ss hms{time_point - dp};
        std::chrono::weekday wd{dp};

        auto remainder_ms =
            std::chrono::duration_cast<std::chrono::milliseconds>(
                hms.subseconds())
                .count();

        return Value::create(
            Value::trusted,
            Map{
                {strings.year,
                 Value::create(Int{static_cast<int>(ymd.year())})},
                {strings.month,
                 Value::create(Int{static_cast<unsigned>(ymd.month())})},
                {strings.day,
                 Value::create(Int{static_cast<unsigned>(ymd.day())})},
                {strings.hour, Value::create(Int{hms.hours().count()})},
                {strings.minute, Value::create(Int{hms.minutes().count()})},
                {strings.second, Value::create(Int{hms.seconds().count()})},
                {strings.ms, Value::create(Int{remainder_ms})},
                {strings.weekday, *weekday_names.at(wd.c_encoding())},
            });
    };

    if (HAS(1))
    {
        auto zt = std::chrono::zoned_time{locate_tz(GET(1, String)), tp};
        return decompose(zt.get_local_time());
    }
    return decompose(tp);
}

BUILTIN(from_components)
{
    REQUIRE_ARGS("datetime.from_components", PARAM("map", TYPES(Map)),
                 OPTIONAL(PARAM("timezone", TYPES(String))));

    const auto& map = GET(0, Map);

    auto get_or = [&](const Value_Ptr& key, Int fallback) -> Int {
        auto it = map.find(key);
        if (it == map.end())
            return fallback;
        if (not it->second->is<Int>())
            throw Frost_Recoverable_Error{fmt::format(
                "datetime.from_components: '{}' must be Int, got {}",
                key->raw_get<String>(), it->second->type_name())};
        return it->second->raw_get<Int>();
    };

    auto y = get_or(strings.year, 1970);
    auto m = get_or(strings.month, 1);
    auto d = get_or(strings.day, 1);
    auto h = get_or(strings.hour, 0);
    auto min = get_or(strings.minute, 0);
    auto s = get_or(strings.second, 0);
    auto ms = get_or(strings.ms, 0);

    auto ymd = std::chrono::year{static_cast<int>(y)}
               / std::chrono::month{static_cast<unsigned>(m)}
               / std::chrono::day{static_cast<unsigned>(d)};

    if (not ymd.ok())
        throw Frost_Recoverable_Error{fmt::format(
            "datetime.from_components: invalid date {}-{}-{}", y, m, d)};

    auto require_range = [](std::string_view name, Int val, Int lo, Int hi) {
        if ((val < lo) || (val > hi))
            throw Frost_Recoverable_Error{fmt::format(
                "datetime.from_components: {} must be {}-{}, got {}", name, lo,
                hi, val)};
    };

    require_range("hour", h, 0, 23);
    require_range("minute", min, 0, 59);
    require_range("second", s, 0, 59);
    require_range("ms", ms, 0, 999);

    auto dur = std::chrono::hours{h} + std::chrono::minutes{min}
               + std::chrono::seconds{s} + std::chrono::milliseconds{ms};

    if (HAS(1))
    {
        auto local_tp = std::chrono::local_days{ymd} + dur;
        auto zt = std::chrono::zoned_time{
            locate_tz(GET(1, String)), local_tp};
        return Value::create(tp_to_millis(
            std::chrono::time_point_cast<std::chrono::milliseconds>(
                zt.get_sys_time())));
    }

    auto tp = std::chrono::sys_days{ymd} + dur;
    return Value::create(tp_to_millis(
        std::chrono::time_point_cast<std::chrono::milliseconds>(tp)));
}

namespace
{

Value_Ptr make_ms_map()
{
    using namespace std::chrono;
    return Value::create(
        Value::trusted,
        Map{
            {strings.second, Value::create(Int{ms_per(seconds{1})})},
            {strings.minute, Value::create(Int{ms_per(minutes{1})})},
            {strings.hour, Value::create(Int{ms_per(hours{1})})},
            {strings.day, Value::create(Int{ms_per(days{1})})},
            {strings.week, Value::create(Int{ms_per(weeks{1})})},
        });
}

} // namespace

} // namespace datetime

STDLIB_MODULE(datetime, ENTRY(now), ENTRY(format), ENTRY(parse),
              ENTRY(components), ENTRY(from_components),
              {"ms"_s, datetime::make_ms_map()},
              {"epoch"_s, Value::create(Int{0})})

} // namespace frst
