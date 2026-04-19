#include "frost/exceptions.hpp"
#include <frost/data-builtin.hpp>
#include <frost/extensions-common.hpp>
#include <limits>

#define TOML_EXCEPTIONS 0
#include <toml++/toml.hpp>

#include <chrono>
#include <sstream>

namespace tomlpp = ::toml;

template <>
struct fmt::formatter<tomlpp::date>
{
    constexpr auto parse(format_parse_context& ctx)
    {
        return ctx.begin();
    }

    auto format(const tomlpp::date& d, format_context& ctx) const
    {
        return fmt::format_to(ctx.out(), "{:04}-{:02}-{:02}", d.year, d.month,
                              d.day);
    }
};

template <>
struct fmt::formatter<tomlpp::time>
{
    constexpr auto parse(format_parse_context& ctx)
    {
        return ctx.begin();
    }

    auto format(const tomlpp::time& t, format_context& ctx) const
    {
        auto out = fmt::format_to(ctx.out(), "{:02}:{:02}:{:02}", t.hour,
                                  t.minute, t.second);
        if (t.nanosecond != 0)
            out = fmt::format_to(out, ".{:09}", t.nanosecond);
        return out;
    }
};

template <>
struct fmt::formatter<tomlpp::time_offset>
{
    constexpr auto parse(format_parse_context& ctx)
    {
        return ctx.begin();
    }

    auto format(const tomlpp::time_offset& o, format_context& ctx) const
    {
        if (o.minutes == 0)
            return fmt::format_to(ctx.out(), "Z");
        auto total = o.minutes;
        auto sign = total < 0 ? '-' : '+';
        if (total < 0)
            total = -total;
        return fmt::format_to(ctx.out(), "{}{:02}:{:02}", sign, total / 60,
                              total % 60);
    }
};

template <>
struct fmt::formatter<tomlpp::date_time>
{
    constexpr auto parse(format_parse_context& ctx)
    {
        return ctx.begin();
    }

    auto format(const tomlpp::date_time& dt, format_context& ctx) const
    {
        auto out = fmt::format_to(ctx.out(), "{}T{}", dt.date, dt.time);
        if (dt.offset)
            out = fmt::format_to(out, "{}", dt.offset.value());
        return out;
    }
};

namespace frst
{
namespace toml
{

namespace
{

Function make_date(const tomlpp::date& date)
{
    auto repr = Value::create(fmt::format("{}", date));

    return std::make_shared<Data_Builtin<tomlpp::date>>(
        [repr = std::move(repr)](builtin_args_t args) {
            REQUIRE_NULLARY("toml.date");
            return repr;
        },
        "toml.date", date);
}

Function make_time(const tomlpp::time& time)
{
    auto repr = Value::create(fmt::format("{}", time));

    return std::make_shared<Data_Builtin<tomlpp::time>>(
        [repr = std::move(repr)](builtin_args_t args) {
            REQUIRE_NULLARY("toml.time");
            return repr;
        },
        "toml.time", time);
}

Function make_datetime(const tomlpp::date_time& datetime)
{
    auto repr = Value::create(fmt::format("{}", datetime));

    return std::make_shared<Data_Builtin<tomlpp::date_time>>(
        [repr = std::move(repr)](builtin_args_t args) {
            REQUIRE_NULLARY("toml.date_time");
            return repr;
        },
        "toml.date_time", datetime);
}

Function make_special_float(const double& d)
{
    // precondition: d is NaN, +Inf, or -Inf

    auto repr = Value::create([&] -> std::string {
        if (std::isnan(d))
        {
            return "nan";
        }
        else if (std::isinf(d))
        {
            if (d > 0)
                return "inf";
            else
                return "-inf";
        }
        THROW_UNREACHABLE;
    }());

    return std::make_shared<Data_Builtin<double>>(
        [repr = std::move(repr)](builtin_args_t args) {
            REQUIRE_NULLARY("toml.special_float");
            return repr;
        },
        "toml.special_float", d);
}

} // namespace

BUILTIN(date)
{
    REQUIRE_ARGS("toml.date", TYPES(String));

    std::chrono::year_month_day ymd;
    std::istringstream iss{GET(0, String)};
    iss >> std::chrono::parse("%F", ymd);
    if (iss.fail())
        throw Frost_Recoverable_Error{
            fmt::format("toml.date: invalid date '{}' (expected YYYY-MM-DD)",
                        GET(0, String))};

    return Value::create(make_date({static_cast<int>(ymd.year()),
                                    static_cast<unsigned>(ymd.month()),
                                    static_cast<unsigned>(ymd.day())}));
}

BUILTIN(time)
{
    REQUIRE_ARGS("toml.time", TYPES(String));

    std::chrono::nanoseconds dur;
    std::istringstream iss{GET(0, String)};
    iss >> std::chrono::parse("%T", dur);
    if (iss.fail())
        throw Frost_Recoverable_Error{
            fmt::format("toml.time: invalid time '{}' (expected "
                        "HH:MM:SS[.nnnnnnnnn])",
                        GET(0, String))};

    std::chrono::hh_mm_ss hms{dur};
    return Value::create(
        make_time({static_cast<unsigned>(hms.hours().count()),
                   static_cast<unsigned>(hms.minutes().count()),
                   static_cast<unsigned>(hms.seconds().count()),
                   static_cast<unsigned>(hms.subseconds().count())}));
}

BUILTIN(date_time)
{
    REQUIRE_ARGS("toml.date_time", TYPES(String));

    auto result = tomlpp::parse("v = " + GET(0, String));
    if (not result)
        throw Frost_Recoverable_Error{
            fmt::format("Invalid TOML datetime: '{}'", GET(0, String))};

    auto* val = result.table()["v"].as<tomlpp::date_time>();
    if (not val)
        throw Frost_Recoverable_Error{
            fmt::format("'{}' is not a datetime", GET(0, String))};

    return Value::create(make_datetime(val->get()));
}

BUILTIN(special_float)
{
    REQUIRE_ARGS("toml.special_float", TYPES(String));

    using lim = std::numeric_limits<double>;

    const auto& str = GET(0, String);
    if (str == "nan")
        return Value::create(make_special_float(lim::quiet_NaN()));
    else if (str == "inf")
        return Value::create(make_special_float(lim::infinity()));
    else if (str == "-inf")
        return Value::create(make_special_float(-lim::infinity()));

    throw Frost_Recoverable_Error(fmt::format(
        "toml.special_float: Input must be 'nan', 'inf', or '-inf', got: {}", str));
}

void append_to(tomlpp::array& parent, const Value_Ptr& val);
void encode_into(tomlpp::table& parent, std::string_view key,
                 const Value_Ptr& val);

template <typename Inserter>
struct Encode_Toml
{
    Inserter insert;

    void operator()(const String& s) const
    {
        insert(s);
    }

    void operator()(const Int& i) const
    {
        insert(i);
    }

    void operator()(const Float& f) const
    {
        insert(f);
    }

    void operator()(const Bool& b) const
    {
        insert(auto{b});
    }

    void operator()(const Null&) const
    {
        throw Frost_Recoverable_Error{
            "toml.encode: TOML does not support null values"};
    }

    void operator()(const Array& arr) const
    {
        tomlpp::array out;
        for (const auto& elem : arr)
            append_to(out, elem);
        insert(std::move(out));
    }

    void operator()(const Map& map) const
    {
        tomlpp::table out;
        for (const auto& [k, v] : map)
        {
            if (not k->is<String>())
                throw Frost_Recoverable_Error{
                    fmt::format("toml.encode: Map key must be String, got {}",
                                k->type_name())};
            encode_into(out, k->raw_get<String>(), v);
        }
        insert(std::move(out));
    }

    void operator()(const Function& fn) const
    {
        if (auto* db =
                dynamic_cast<const Data_Builtin<tomlpp::date>*>(fn.get()))
            insert(db->data());
        else if (auto* db =
                     dynamic_cast<const Data_Builtin<tomlpp::time>*>(fn.get()))
            insert(db->data());
        else if (auto* db =
                     dynamic_cast<const Data_Builtin<tomlpp::date_time>*>(
                         fn.get()))
            insert(db->data());
        else if (auto* db = dynamic_cast<const Data_Builtin<double>*>(fn.get()))
            insert(db->data());
        else
            throw Frost_Recoverable_Error{
                "toml.encode: cannot serialize Function to TOML"};
    }
};

void encode_into(tomlpp::table& parent, std::string_view key,
                 const Value_Ptr& val)
{
    val->visit(Encode_Toml{[&](auto&& v) {
        parent.insert_or_assign(key, std::forward<decltype(v)>(v));
    }});
}

void append_to(tomlpp::array& parent, const Value_Ptr& val)
{
    val->visit(Encode_Toml{[&](auto&& v) {
        parent.push_back(std::forward<decltype(v)>(v));
    }});
}

BUILTIN(encode)
{
    REQUIRE_ARGS("toml.encode", TYPES(Map));

    tomlpp::table root;
    for (const auto& [k, v] : GET(0, Map))
    {
        if (not k->is<String>())
            throw Frost_Recoverable_Error{fmt::format(
                "toml.encode: Map key must be String, got {}", k->type_name())};
        encode_into(root, k->raw_get<String>(), v);
    }

    std::ostringstream oss;
    oss << root;
    return Value::create(String{oss.str()});
}

struct Decode_Toml
{

    template <typename T>
        requires std::same_as<T, std::string>
                 || std::same_as<T, std::int64_t>
                 || std::same_as<T, bool>
    static Value_Ptr operator()(const tomlpp::value<T>& value)
    {
        return Value::create(auto{value.get()});
    }

    static Value_Ptr operator()(const tomlpp::value<double>& value)
    {
        double d = value.get();
        if (std::isnan(d) || std::isinf(d))
            return Value::create(make_special_float(d));
        return Value::create(d);
    }

    static Value_Ptr operator()(const tomlpp::array& arr)
    {
        return Value::create(arr
                             | std::views::transform([&](const auto& elem) {
                                   return elem.visit(Decode_Toml{});
                               })
                             | std::ranges::to<Array>());
    }

    static Value_Ptr operator()(const tomlpp::table& table)
    {
        Map result;
        for (const auto& [k, v] : table)
        {
            result.try_emplace(Value::create(String{k}),
                               v.visit(Decode_Toml{}));
        }
        return Value::create(Value::trusted, std::move(result));
    }

    static Value_Ptr operator()(const tomlpp::value<tomlpp::date>& date)
    {
        return Value::create(make_date(date.get()));
    }

    static Value_Ptr operator()(const tomlpp::value<tomlpp::time>& time)
    {
        return Value::create(make_time(time.get()));
    }

    static Value_Ptr operator()(
        const tomlpp::value<tomlpp::date_time>& datetime)
    {
        return Value::create(make_datetime(datetime.get()));
    }

} constexpr static decode_toml;

BUILTIN(decode)
{
    REQUIRE_ARGS("toml.decode", TYPES(String));

    auto result = tomlpp::parse(GET(0, String));
    if (not result)
        throw Frost_Recoverable_Error{
            fmt::format("toml.decode: {}", result.error().description())};

    return result.table().visit(decode_toml);
}

} // namespace toml

REGISTER_EXTENSION(toml, ENTRY(decode), ENTRY(encode), ENTRY(date), ENTRY(time),
                   ENTRY(date_time), ENTRY(special_float));

} // namespace frst
