#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/builtin.hpp>
#include <frost/stdlib.hpp>
#include <frost/value.hpp>

using namespace frst;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;

namespace
{

Map datetime_module()
{
    Stdlib_Registry_Builder builder;
    register_module_datetime(builder);
    auto registry = std::move(builder).build();
    auto module = registry.lookup_module("std.datetime");
    REQUIRE(module.has_value());
    REQUIRE(module.value()->is<Map>());
    return module.value()->raw_get<Map>();
}

Function lookup(const Map& mod, const std::string& name)
{
    auto key = Value::create(String{name});
    auto it = mod.find(key);
    REQUIRE(it != mod.end());
    REQUIRE(it->second->is<Function>());
    return it->second->raw_get<Function>();
}

Value_Ptr lookup_value(const Map& mod, const std::string& name)
{
    auto key = Value::create(String{name});
    auto it = mod.find(key);
    REQUIRE(it != mod.end());
    return it->second;
}

Value_Ptr get_field(const Value_Ptr& map, const std::string& key)
{
    return map->raw_get<Map>().at(Value::create(String{key}));
}

} // namespace

TEST_CASE("datetime.now")
{
    auto mod = datetime_module();
    auto now = lookup(mod, "now");

    SECTION("returns a positive Int")
    {
        auto result = now->call({});
        REQUIRE(result->is<Int>());
        CHECK(result->raw_get<Int>() > 0);
    }

    SECTION("two calls are non-decreasing")
    {
        auto a = now->call({})->raw_get<Int>();
        auto b = now->call({})->raw_get<Int>();
        CHECK(b >= a);
    }

    SECTION("arity")
    {
        CHECK_THROWS_WITH(now->call({Value::create(1_f)}),
                          ContainsSubstring("too many"));
    }
}

TEST_CASE("datetime.format")
{
    auto mod = datetime_module();
    auto format = lookup(mod, "format");

    SECTION("formats a known timestamp")
    {
        // 2024-03-16 00:00:00 UTC = 1710547200000 ms
        auto result = format->call(
            {Value::create(1710547200000_f), Value::create("%Y-%m-%d"s)});
        CHECK(result->raw_get<String>() == "2024-03-16");
    }

    SECTION("formats with time")
    {
        // 2024-03-16 14:30:00 UTC
        auto millis = 1710547200000_f + 14 * 3600000 + 30 * 60000;
        auto result = format->call(
            {Value::create(millis), Value::create("%H:%M"s)});
        CHECK(result->raw_get<String>() == "14:30");
    }

    SECTION("epoch formats correctly")
    {
        auto result = format->call(
            {Value::create(0_f), Value::create("%Y-%m-%d"s)});
        CHECK(result->raw_get<String>() == "1970-01-01");
    }

    SECTION("invalid pattern")
    {
        CHECK_THROWS_WITH(
            format->call({Value::create(0_f), Value::create("%Q"s)}),
            ContainsSubstring("datetime.format"));
    }

    SECTION("arity")
    {
        CHECK_THROWS_WITH(format->call({}),
                          ContainsSubstring("insufficient"));
        CHECK_THROWS_WITH(format->call({Value::create(0_f)}),
                          ContainsSubstring("insufficient"));
        CHECK_THROWS_WITH(
            format->call(
                {Value::create(0_f), Value::create("%Y"s), Value::create(0_f)}),
            ContainsSubstring("too many"));
    }

    SECTION("type constraints")
    {
        CHECK_THROWS_WITH(
            format->call({Value::create("x"s), Value::create("%Y"s)}),
            ContainsSubstring("Int"));
        CHECK_THROWS_WITH(
            format->call({Value::create(0_f), Value::create(42_f)}),
            ContainsSubstring("String"));
    }
}

TEST_CASE("datetime.parse")
{
    auto mod = datetime_module();
    auto parse = lookup(mod, "parse");

    SECTION("parses a date")
    {
        auto result = parse->call(
            {Value::create("2024-03-16"s), Value::create("%Y-%m-%d"s)});
        CHECK(result->raw_get<Int>() == 1710547200000);
    }

    SECTION("parses date and time")
    {
        auto result = parse->call(
            {Value::create("2024-03-16 14:30:00"s),
             Value::create("%Y-%m-%d %H:%M:%S"s)});
        auto expected = 1710547200000_f + 14 * 3600000 + 30 * 60000;
        CHECK(result->raw_get<Int>() == expected);
    }

    SECTION("round-trips with format")
    {
        auto format = lookup(mod, "format");
        auto millis = Value::create(1710547200000_f);
        auto pattern = Value::create("%Y-%m-%d %H:%M:%S"s);
        auto formatted = format->call({millis, pattern});
        auto parsed = parse->call({formatted, pattern});
        CHECK(parsed->raw_get<Int>() == 1710547200000);
    }

    SECTION("invalid input")
    {
        CHECK_THROWS_WITH(
            parse->call({Value::create("not-a-date"s),
                         Value::create("%Y-%m-%d"s)}),
            ContainsSubstring("does not match"));
    }

    SECTION("arity")
    {
        CHECK_THROWS_WITH(parse->call({}),
                          ContainsSubstring("insufficient"));
        CHECK_THROWS_WITH(parse->call({Value::create("x"s)}),
                          ContainsSubstring("insufficient"));
        CHECK_THROWS_WITH(
            parse->call({Value::create("x"s), Value::create("%Y"s),
                         Value::create(0_f)}),
            ContainsSubstring("too many"));
    }

    SECTION("type constraints")
    {
        CHECK_THROWS_WITH(
            parse->call({Value::create(42_f), Value::create("%Y"s)}),
            ContainsSubstring("String"));
        CHECK_THROWS_WITH(
            parse->call({Value::create("x"s), Value::create(42_f)}),
            ContainsSubstring("String"));
    }
}

TEST_CASE("datetime.components")
{
    auto mod = datetime_module();
    auto components = lookup(mod, "components");

    SECTION("epoch components")
    {
        auto result = components->call({Value::create(0_f)});
        REQUIRE(result->is<Map>());
        CHECK(get_field(result, "year")->raw_get<Int>() == 1970);
        CHECK(get_field(result, "month")->raw_get<Int>() == 1);
        CHECK(get_field(result, "day")->raw_get<Int>() == 1);
        CHECK(get_field(result, "hour")->raw_get<Int>() == 0);
        CHECK(get_field(result, "minute")->raw_get<Int>() == 0);
        CHECK(get_field(result, "second")->raw_get<Int>() == 0);
        CHECK(get_field(result, "ms")->raw_get<Int>() == 0);
        CHECK(get_field(result, "weekday")->raw_get<String>() == "Thursday");
    }

    SECTION("known date")
    {
        // 2024-03-16 = Saturday
        auto result = components->call({Value::create(1710547200000_f)});
        CHECK(get_field(result, "year")->raw_get<Int>() == 2024);
        CHECK(get_field(result, "month")->raw_get<Int>() == 3);
        CHECK(get_field(result, "day")->raw_get<Int>() == 16);
        CHECK(get_field(result, "weekday")->raw_get<String>() == "Saturday");
    }

    SECTION("with time and milliseconds")
    {
        // epoch + 14h 30m 15s 500ms
        auto millis = 14 * 3600000_f + 30 * 60000 + 15 * 1000 + 500;
        auto result = components->call({Value::create(millis)});
        CHECK(get_field(result, "hour")->raw_get<Int>() == 14);
        CHECK(get_field(result, "minute")->raw_get<Int>() == 30);
        CHECK(get_field(result, "second")->raw_get<Int>() == 15);
        CHECK(get_field(result, "ms")->raw_get<Int>() == 500);
    }

    SECTION("arity")
    {
        CHECK_THROWS_WITH(components->call({}),
                          ContainsSubstring("insufficient"));
        CHECK_THROWS_WITH(
            components->call({Value::create(0_f), Value::create(0_f)}),
            ContainsSubstring("too many"));
    }

    SECTION("type constraint")
    {
        CHECK_THROWS_WITH(components->call({Value::create("x"s)}),
                          ContainsSubstring("Int"));
    }
}

TEST_CASE("datetime.from_components")
{
    auto mod = datetime_module();
    auto from = lookup(mod, "from_components");

    SECTION("full date")
    {
        auto result = from->call({Value::create(Value::trusted,
            Map{{"year"_s, Value::create(2024_f)},
                {"month"_s, Value::create(3_f)},
                {"day"_s, Value::create(16_f)}})});
        CHECK(result->raw_get<Int>() == 1710547200000);
    }

    SECTION("with time")
    {
        auto result = from->call({Value::create(Value::trusted,
            Map{{"year"_s, Value::create(2024_f)},
                {"month"_s, Value::create(3_f)},
                {"day"_s, Value::create(16_f)},
                {"hour"_s, Value::create(14_f)},
                {"minute"_s, Value::create(30_f)}})});
        auto expected = 1710547200000_f + 14 * 3600000 + 30 * 60000;
        CHECK(result->raw_get<Int>() == expected);
    }

    SECTION("missing fields default to minimum")
    {
        auto result = from->call({Value::create(Value::trusted,
            Map{{"year"_s, Value::create(1970_f)}})});
        CHECK(result->raw_get<Int>() == 0);
    }

    SECTION("round-trips with components")
    {
        auto components = lookup(mod, "components");
        auto millis = Value::create(1710547200000_f);
        auto comp = components->call({millis});
        auto back = from->call({comp});
        CHECK(back->raw_get<Int>() == 1710547200000);
    }

    SECTION("invalid date rejected")
    {
        CHECK_THROWS_WITH(
            from->call({Value::create(Value::trusted,
                Map{{"year"_s, Value::create(2024_f)},
                    {"month"_s, Value::create(2_f)},
                    {"day"_s, Value::create(30_f)}})}),
            ContainsSubstring("invalid date"));
    }

    SECTION("weekday field is ignored")
    {
        // components output includes weekday, but from_components should
        // ignore it (weekday is derived, not an input)
        auto result = from->call({Value::create(Value::trusted,
            Map{{"year"_s, Value::create(2024_f)},
                {"month"_s, Value::create(3_f)},
                {"day"_s, Value::create(16_f)},
                {"weekday"_s, Value::create("bogus"s)}})});
        CHECK(result->raw_get<Int>() == 1710547200000);
    }

    SECTION("non-Int field value rejected")
    {
        CHECK_THROWS_WITH(
            from->call({Value::create(Value::trusted,
                Map{{"year"_s, Value::create("2024"s)}})}),
            ContainsSubstring("must be Int"));
    }

    SECTION("arity")
    {
        CHECK_THROWS_WITH(from->call({}),
                          ContainsSubstring("insufficient"));
        CHECK_THROWS_WITH(
            from->call({Value::create(Value::trusted, Map{}),
                        Value::create(0_f)}),
            ContainsSubstring("too many"));
    }

    SECTION("type constraint")
    {
        CHECK_THROWS_WITH(from->call({Value::create("x"s)}),
                          ContainsSubstring("Map"));
    }
}

TEST_CASE("datetime: constants")
{
    auto mod = datetime_module();

    SECTION("epoch")
    {
        auto epoch = lookup_value(mod, "epoch");
        CHECK(epoch->raw_get<Int>() == 0);
    }

    SECTION("ms sub-map")
    {
        auto ms = lookup_value(mod, "ms");
        REQUIRE(ms->is<Map>());
        const auto& m = ms->raw_get<Map>();

        CHECK(m.at(Value::create("second"s))->raw_get<Int>() == 1000);
        CHECK(m.at(Value::create("minute"s))->raw_get<Int>() == 60'000);
        CHECK(m.at(Value::create("hour"s))->raw_get<Int>() == 3'600'000);
        CHECK(m.at(Value::create("day"s))->raw_get<Int>() == 86'400'000);
        CHECK(m.at(Value::create("week"s))->raw_get<Int>() == 604'800'000);
    }
}

TEST_CASE("datetime: edge cases")
{
    auto mod = datetime_module();
    auto components = lookup(mod, "components");
    auto from = lookup(mod, "from_components");
    auto format = lookup(mod, "format");
    auto parse = lookup(mod, "parse");

    SECTION("pre-epoch timestamp")
    {
        // -1 ms = 1969-12-31 23:59:59.999
        auto result = components->call({Value::create(-1_f)});
        CHECK(get_field(result, "year")->raw_get<Int>() == 1969);
        CHECK(get_field(result, "month")->raw_get<Int>() == 12);
        CHECK(get_field(result, "day")->raw_get<Int>() == 31);
        CHECK(get_field(result, "hour")->raw_get<Int>() == 23);
        CHECK(get_field(result, "minute")->raw_get<Int>() == 59);
        CHECK(get_field(result, "second")->raw_get<Int>() == 59);
        CHECK(get_field(result, "ms")->raw_get<Int>() == 999);
        CHECK(get_field(result, "weekday")->raw_get<String>() == "Wednesday");
    }

    SECTION("pre-epoch from_components")
    {
        auto result = from->call({Value::create(Value::trusted,
            Map{{"year"_s, Value::create(1969_f)},
                {"month"_s, Value::create(12_f)},
                {"day"_s, Value::create(31_f)}})});
        CHECK(result->raw_get<Int>() == -86'400'000);
    }

    SECTION("pre-epoch format")
    {
        auto result = format->call(
            {Value::create(-86'400'000_f), Value::create("%Y-%m-%d"s)});
        CHECK(result->raw_get<String>() == "1969-12-31");
    }

    SECTION("leap year: Feb 29 on leap year")
    {
        auto result = from->call({Value::create(Value::trusted,
            Map{{"year"_s, Value::create(2024_f)},
                {"month"_s, Value::create(2_f)},
                {"day"_s, Value::create(29_f)}})});
        auto comp = components->call({result});
        CHECK(get_field(comp, "month")->raw_get<Int>() == 2);
        CHECK(get_field(comp, "day")->raw_get<Int>() == 29);
    }

    SECTION("leap year: Feb 29 on non-leap year")
    {
        CHECK_THROWS_WITH(
            from->call({Value::create(Value::trusted,
                Map{{"year"_s, Value::create(2023_f)},
                    {"month"_s, Value::create(2_f)},
                    {"day"_s, Value::create(29_f)}})}),
            ContainsSubstring("invalid date"));
    }

    SECTION("invalid month: 0")
    {
        CHECK_THROWS_WITH(
            from->call({Value::create(Value::trusted,
                Map{{"year"_s, Value::create(2024_f)},
                    {"month"_s, Value::create(0_f)}})}),
            ContainsSubstring("invalid date"));
    }

    SECTION("invalid month: 13")
    {
        CHECK_THROWS_WITH(
            from->call({Value::create(Value::trusted,
                Map{{"year"_s, Value::create(2024_f)},
                    {"month"_s, Value::create(13_f)}})}),
            ContainsSubstring("invalid date"));
    }

    SECTION("invalid day: 0")
    {
        CHECK_THROWS_WITH(
            from->call({Value::create(Value::trusted,
                Map{{"year"_s, Value::create(2024_f)},
                    {"month"_s, Value::create(1_f)},
                    {"day"_s, Value::create(0_f)}})}),
            ContainsSubstring("invalid date"));
    }

    SECTION("empty map gives epoch")
    {
        auto result = from->call(
            {Value::create(Value::trusted, Map{})});
        CHECK(result->raw_get<Int>() == 0);
    }

    SECTION("far future: year 3000")
    {
        auto millis = from->call({Value::create(Value::trusted,
            Map{{"year"_s, Value::create(3000_f)},
                {"month"_s, Value::create(1_f)},
                {"day"_s, Value::create(1_f)}})});
        auto comp = components->call({millis});
        CHECK(get_field(comp, "year")->raw_get<Int>() == 3000);
        CHECK(get_field(comp, "month")->raw_get<Int>() == 1);
        CHECK(get_field(comp, "day")->raw_get<Int>() == 1);
    }

    SECTION("millisecond precision round-trip")
    {
        auto millis = from->call({Value::create(Value::trusted,
            Map{{"year"_s, Value::create(2024_f)},
                {"month"_s, Value::create(6_f)},
                {"day"_s, Value::create(15_f)},
                {"hour"_s, Value::create(12_f)},
                {"minute"_s, Value::create(30_f)},
                {"second"_s, Value::create(45_f)},
                {"ms"_s, Value::create(123_f)}})});
        auto comp = components->call({millis});
        CHECK(get_field(comp, "ms")->raw_get<Int>() == 123);
    }

    SECTION("parse empty string")
    {
        CHECK_THROWS_WITH(
            parse->call({Value::create(""s), Value::create("%Y-%m-%d"s)}),
            ContainsSubstring("does not match"));
    }

    SECTION("format with empty pattern gives default representation")
    {
        auto result = format->call(
            {Value::create(0_f), Value::create(""s)});
        CHECK(result->raw_get<String>() == "1970-01-01 00:00:00.000");
    }

    SECTION("hour out of range")
    {
        CHECK_THROWS_WITH(
            from->call({Value::create(Value::trusted,
                Map{{"year"_s, Value::create(2024_f)},
                    {"hour"_s, Value::create(24_f)}})}),
            ContainsSubstring("hour"));
    }

    SECTION("negative hour")
    {
        CHECK_THROWS_WITH(
            from->call({Value::create(Value::trusted,
                Map{{"year"_s, Value::create(2024_f)},
                    {"hour"_s, Value::create(-1_f)}})}),
            ContainsSubstring("hour"));
    }

    SECTION("minute out of range")
    {
        CHECK_THROWS_WITH(
            from->call({Value::create(Value::trusted,
                Map{{"year"_s, Value::create(2024_f)},
                    {"minute"_s, Value::create(60_f)}})}),
            ContainsSubstring("minute"));
    }

    SECTION("second out of range")
    {
        CHECK_THROWS_WITH(
            from->call({Value::create(Value::trusted,
                Map{{"year"_s, Value::create(2024_f)},
                    {"second"_s, Value::create(60_f)}})}),
            ContainsSubstring("second"));
    }

    SECTION("ms out of range")
    {
        CHECK_THROWS_WITH(
            from->call({Value::create(Value::trusted,
                Map{{"year"_s, Value::create(2024_f)},
                    {"ms"_s, Value::create(1000_f)}})}),
            ContainsSubstring("ms"));
    }

    SECTION("boundary values accepted")
    {
        // 23:59:59.999 should be valid
        auto result = from->call({Value::create(Value::trusted,
            Map{{"year"_s, Value::create(2024_f)},
                {"hour"_s, Value::create(23_f)},
                {"minute"_s, Value::create(59_f)},
                {"second"_s, Value::create(59_f)},
                {"ms"_s, Value::create(999_f)}})});
        auto comp = components->call({result});
        CHECK(get_field(comp, "hour")->raw_get<Int>() == 23);
        CHECK(get_field(comp, "minute")->raw_get<Int>() == 59);
        CHECK(get_field(comp, "second")->raw_get<Int>() == 59);
        CHECK(get_field(comp, "ms")->raw_get<Int>() == 999);
    }

    SECTION("unknown keys in from_components are ignored")
    {
        auto result = from->call({Value::create(Value::trusted,
            Map{{"year"_s, Value::create(2024_f)},
                {"month"_s, Value::create(3_f)},
                {"day"_s, Value::create(16_f)},
                {"foo"_s, Value::create("bar"s)}})});
        CHECK(result->raw_get<Int>() == 1710547200000);
    }
}
