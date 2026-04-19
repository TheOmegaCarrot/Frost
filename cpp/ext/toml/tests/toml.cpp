#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/data-builtin.hpp>
#include <frost/extensions-common.hpp>
#include <frost/value.hpp>

#define TOML_EXCEPTIONS 0
#include <toml++/toml.hpp>

namespace tomlpp = ::toml;

using namespace frst;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;

namespace frst
{
void register_module_toml(Stdlib_Registry_Builder&);
}

namespace
{

Map toml_module()
{
    Stdlib_Registry_Builder builder;
    register_module_toml(builder);
    auto registry = std::move(builder).build();
    auto module = registry.lookup_module("ext.toml");
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

} // namespace

TEST_CASE("toml.decode: primitives")
{
    auto mod = toml_module();
    auto decode = lookup(mod, "decode");

    SECTION("string")
    {
        auto result = decode->call({Value::create(R"(key = "hello")"s)});
        REQUIRE(result->is<Map>());
        auto val = result->raw_get<Map>().at(Value::create("key"s));
        CHECK(val->raw_get<String>() == "hello");
    }

    SECTION("integer")
    {
        auto result = decode->call({Value::create("key = 42"s)});
        auto val = result->raw_get<Map>().at(Value::create("key"s));
        CHECK(val->raw_get<Int>() == 42);
    }

    SECTION("negative integer")
    {
        auto result = decode->call({Value::create("key = -100"s)});
        auto val = result->raw_get<Map>().at(Value::create("key"s));
        CHECK(val->raw_get<Int>() == -100);
    }

    SECTION("float")
    {
        auto result = decode->call({Value::create("key = 3.14"s)});
        auto val = result->raw_get<Map>().at(Value::create("key"s));
        CHECK(val->raw_get<Float>() == Catch::Approx(3.14));
    }

    SECTION("boolean true")
    {
        auto result = decode->call({Value::create("key = true"s)});
        auto val = result->raw_get<Map>().at(Value::create("key"s));
        CHECK(val->raw_get<Bool>() == true);
    }

    SECTION("boolean false")
    {
        auto result = decode->call({Value::create("key = false"s)});
        auto val = result->raw_get<Map>().at(Value::create("key"s));
        CHECK(val->raw_get<Bool>() == false);
    }
}

TEST_CASE("toml.decode: arrays and tables")
{
    auto mod = toml_module();
    auto decode = lookup(mod, "decode");

    SECTION("array of integers")
    {
        auto result = decode->call({Value::create("key = [1, 2, 3]"s)});
        auto arr = result->raw_get<Map>().at(Value::create("key"s));
        REQUIRE(arr->is<Array>());
        const auto& elems = arr->raw_get<Array>();
        REQUIRE(elems.size() == 3);
        CHECK(elems[0]->raw_get<Int>() == 1);
        CHECK(elems[1]->raw_get<Int>() == 2);
        CHECK(elems[2]->raw_get<Int>() == 3);
    }

    SECTION("nested table")
    {
        auto result = decode->call({Value::create(R"(
[server]
host = "localhost"
port = 8080
)"s)});
        auto server = result->raw_get<Map>().at(Value::create("server"s));
        REQUIRE(server->is<Map>());
        CHECK(server->raw_get<Map>().at(Value::create("host"s))->raw_get<String>()
              == "localhost");
        CHECK(server->raw_get<Map>().at(Value::create("port"s))->raw_get<Int>()
              == 8080);
    }

    SECTION("array of tables")
    {
        auto result = decode->call({Value::create(R"(
[[items]]
name = "a"

[[items]]
name = "b"
)"s)});
        auto items = result->raw_get<Map>().at(Value::create("items"s));
        REQUIRE(items->is<Array>());
        const auto& arr = items->raw_get<Array>();
        REQUIRE(arr.size() == 2);
        CHECK(arr[0]->raw_get<Map>().at(Value::create("name"s))->raw_get<String>()
              == "a");
        CHECK(arr[1]->raw_get<Map>().at(Value::create("name"s))->raw_get<String>()
              == "b");
    }
}

TEST_CASE("toml.decode: dates and times")
{
    auto mod = toml_module();
    auto decode = lookup(mod, "decode");

    SECTION("local date")
    {
        auto result = decode->call({Value::create("d = 1979-05-27"s)});
        auto val = result->raw_get<Map>().at(Value::create("d"s));
        REQUIRE(val->is<Function>());

        // calling it returns the string representation
        CHECK(val->raw_get<Function>()->call({})->raw_get<String>()
              == "1979-05-27");

        // Data_Builtin carries the toml++ date
        auto* db = dynamic_cast<const Data_Builtin<tomlpp::date>*>(
            val->raw_get<Function>().get());
        REQUIRE(db != nullptr);
        CHECK(db->data().year == 1979);
        CHECK(db->data().month == 5);
        CHECK(db->data().day == 27);
    }

    SECTION("local time")
    {
        auto result = decode->call({Value::create("t = 07:32:00"s)});
        auto val = result->raw_get<Map>().at(Value::create("t"s));
        REQUIRE(val->is<Function>());
        CHECK(val->raw_get<Function>()->call({})->raw_get<String>()
              == "07:32:00");

        auto* db = dynamic_cast<const Data_Builtin<tomlpp::time>*>(
            val->raw_get<Function>().get());
        REQUIRE(db != nullptr);
        CHECK(db->data().hour == 7);
        CHECK(db->data().minute == 32);
        CHECK(db->data().second == 0);
    }

    SECTION("local time with fractional seconds")
    {
        auto result = decode->call({Value::create("t = 00:32:00.999999"s)});
        auto val = result->raw_get<Map>().at(Value::create("t"s));
        REQUIRE(val->is<Function>());
        CHECK(val->raw_get<Function>()->call({})->raw_get<String>()
              == "00:32:00.999999000");

        auto* db = dynamic_cast<const Data_Builtin<tomlpp::time>*>(
            val->raw_get<Function>().get());
        REQUIRE(db != nullptr);
        CHECK(db->data().nanosecond == 999999000);
    }

    SECTION("offset datetime")
    {
        auto result =
            decode->call({Value::create("dt = 1979-05-27T07:32:00Z"s)});
        auto val = result->raw_get<Map>().at(Value::create("dt"s));
        REQUIRE(val->is<Function>());
        CHECK(val->raw_get<Function>()->call({})->raw_get<String>()
              == "1979-05-27T07:32:00Z");

        auto* db = dynamic_cast<const Data_Builtin<tomlpp::date_time>*>(
            val->raw_get<Function>().get());
        REQUIRE(db != nullptr);
        CHECK(db->data().date.year == 1979);
        CHECK(db->data().offset.has_value());
        CHECK(db->data().offset.value().minutes == 0);
    }

    SECTION("offset datetime with timezone")
    {
        auto result =
            decode->call({Value::create("dt = 1979-05-27T07:32:00-05:00"s)});
        auto val = result->raw_get<Map>().at(Value::create("dt"s));
        CHECK(val->raw_get<Function>()->call({})->raw_get<String>()
              == "1979-05-27T07:32:00-05:00");

        auto* db = dynamic_cast<const Data_Builtin<tomlpp::date_time>*>(
            val->raw_get<Function>().get());
        REQUIRE(db != nullptr);
        CHECK(db->data().offset.value().minutes == -300);
    }

    SECTION("local datetime (no offset)")
    {
        auto result =
            decode->call({Value::create("dt = 1979-05-27T07:32:00"s)});
        auto val = result->raw_get<Map>().at(Value::create("dt"s));
        CHECK(val->raw_get<Function>()->call({})->raw_get<String>()
              == "1979-05-27T07:32:00");

        auto* db = dynamic_cast<const Data_Builtin<tomlpp::date_time>*>(
            val->raw_get<Function>().get());
        REQUIRE(db != nullptr);
        CHECK(not db->data().offset.has_value());
    }
}

TEST_CASE("toml.decode: special floats")
{
    auto mod = toml_module();
    auto decode = lookup(mod, "decode");

    SECTION("nan")
    {
        auto result = decode->call({Value::create("v = nan"s)});
        auto val = result->raw_get<Map>().at(Value::create("v"s));
        REQUIRE(val->is<Function>());
        CHECK(val->raw_get<Function>()->call({})->raw_get<String>() == "nan");

        auto* db = dynamic_cast<const Data_Builtin<double>*>(
            val->raw_get<Function>().get());
        REQUIRE(db != nullptr);
        CHECK(std::isnan(db->data()));
    }

    SECTION("inf")
    {
        auto result = decode->call({Value::create("v = inf"s)});
        auto val = result->raw_get<Map>().at(Value::create("v"s));
        CHECK(val->raw_get<Function>()->call({})->raw_get<String>() == "inf");

        auto* db = dynamic_cast<const Data_Builtin<double>*>(
            val->raw_get<Function>().get());
        REQUIRE(db != nullptr);
        CHECK(std::isinf(db->data()));
        CHECK(db->data() > 0);
    }

    SECTION("-inf")
    {
        auto result = decode->call({Value::create("v = -inf"s)});
        auto val = result->raw_get<Map>().at(Value::create("v"s));
        CHECK(val->raw_get<Function>()->call({})->raw_get<String>() == "-inf");

        auto* db = dynamic_cast<const Data_Builtin<double>*>(
            val->raw_get<Function>().get());
        REQUIRE(db != nullptr);
        CHECK(std::isinf(db->data()));
        CHECK(db->data() < 0);
    }
}

TEST_CASE("toml.decode: complex document")
{
    auto mod = toml_module();
    auto decode = lookup(mod, "decode");

    auto result = decode->call({Value::create(R"(
title = "TOML Example"

[owner]
name = "Tom Preston-Werner"
dob = 1979-05-27T07:32:00-08:00

[database]
enabled = true
ports = [8001, 8001, 8002]
temp_targets = { cpu = 79.5, case = 72.0 }
)"s)});

    REQUIRE(result->is<Map>());
    const auto& root = result->raw_get<Map>();

    CHECK(root.at(Value::create("title"s))->raw_get<String>()
          == "TOML Example");

    auto owner = root.at(Value::create("owner"s));
    CHECK(owner->raw_get<Map>().at(Value::create("name"s))->raw_get<String>()
          == "Tom Preston-Werner");

    // dob is a Data_Builtin datetime
    auto dob = owner->raw_get<Map>().at(Value::create("dob"s));
    REQUIRE(dob->is<Function>());
    CHECK(dob->raw_get<Function>()->call({})->raw_get<String>()
          == "1979-05-27T07:32:00-08:00");

    auto db = root.at(Value::create("database"s));
    CHECK(db->raw_get<Map>().at(Value::create("enabled"s))->raw_get<Bool>()
          == true);

    auto ports = db->raw_get<Map>().at(Value::create("ports"s));
    REQUIRE(ports->raw_get<Array>().size() == 3);

    auto temps = db->raw_get<Map>().at(Value::create("temp_targets"s));
    CHECK(temps->raw_get<Map>().at(Value::create("cpu"s))->raw_get<Float>()
          == Catch::Approx(79.5));
}

TEST_CASE("toml.decode: arity")
{
    auto mod = toml_module();
    auto decode = lookup(mod, "decode");

    SECTION("too few arguments")
    {
        CHECK_THROWS_WITH(decode->call({}),
                          ContainsSubstring("insufficient arguments"));
    }

    SECTION("too many arguments")
    {
        CHECK_THROWS_WITH(
            decode->call({Value::create("x = 1"s), Value::create("y = 2"s)}),
            ContainsSubstring("too many arguments"));
    }

    SECTION("wrong type")
    {
        CHECK_THROWS_WITH(decode->call({Value::create(42_f)}),
                          ContainsSubstring("String"));
    }
}

TEST_CASE("toml.decode: parse error")
{
    auto mod = toml_module();
    auto decode = lookup(mod, "decode");

    CHECK_THROWS_WITH(decode->call({Value::create("= invalid"s)}),
                      ContainsSubstring("toml.decode"));
}

TEST_CASE("toml.date: constructor")
{
    auto mod = toml_module();
    auto date_fn = lookup(mod, "date");

    SECTION("valid date")
    {
        auto result = date_fn->call({Value::create("2024-03-16"s)});
        REQUIRE(result->is<Function>());
        CHECK(result->raw_get<Function>()->call({})->raw_get<String>()
              == "2024-03-16");

        auto* db = dynamic_cast<const Data_Builtin<tomlpp::date>*>(
            result->raw_get<Function>().get());
        REQUIRE(db != nullptr);
        CHECK(db->data().year == 2024);
        CHECK(db->data().month == 3);
        CHECK(db->data().day == 16);
    }

    SECTION("invalid date")
    {
        CHECK_THROWS_WITH(date_fn->call({Value::create("not-a-date"s)}),
                          ContainsSubstring("toml.date"));
    }

    SECTION("arity")
    {
        CHECK_THROWS_WITH(date_fn->call({}),
                          ContainsSubstring("insufficient arguments"));
    }
}

TEST_CASE("toml.time: constructor")
{
    auto mod = toml_module();
    auto time_fn = lookup(mod, "time");

    SECTION("valid time")
    {
        auto result = time_fn->call({Value::create("14:30:00"s)});
        REQUIRE(result->is<Function>());
        CHECK(result->raw_get<Function>()->call({})->raw_get<String>()
              == "14:30:00");
    }

    SECTION("with fractional seconds")
    {
        auto result = time_fn->call({Value::create("14:30:00.123456789"s)});
        CHECK(result->raw_get<Function>()->call({})->raw_get<String>()
              == "14:30:00.123456789");

        auto* db = dynamic_cast<const Data_Builtin<tomlpp::time>*>(
            result->raw_get<Function>().get());
        REQUIRE(db != nullptr);
        CHECK(db->data().nanosecond == 123456789);
    }

    SECTION("invalid time")
    {
        CHECK_THROWS_WITH(time_fn->call({Value::create("25:00:00"s)}),
                          ContainsSubstring("toml.time"));
    }

    SECTION("missing seconds rejected")
    {
        CHECK_THROWS_WITH(time_fn->call({Value::create("12:30"s)}),
                          ContainsSubstring("toml.time"));
    }
}

TEST_CASE("toml.date_time: constructor")
{
    auto mod = toml_module();
    auto datetime_fn = lookup(mod, "date_time");

    SECTION("offset datetime")
    {
        auto result =
            datetime_fn->call({Value::create("2024-01-15T10:30:00Z"s)});
        REQUIRE(result->is<Function>());
        CHECK(result->raw_get<Function>()->call({})->raw_get<String>()
              == "2024-01-15T10:30:00Z");
    }

    SECTION("local datetime")
    {
        auto result =
            datetime_fn->call({Value::create("2024-01-15T10:30:00"s)});
        CHECK(result->raw_get<Function>()->call({})->raw_get<String>()
              == "2024-01-15T10:30:00");

        auto* db = dynamic_cast<const Data_Builtin<tomlpp::date_time>*>(
            result->raw_get<Function>().get());
        REQUIRE(db != nullptr);
        CHECK(not db->data().offset.has_value());
    }

    SECTION("with timezone offset")
    {
        auto result =
            datetime_fn->call({Value::create("2024-01-15T10:30:00+05:30"s)});
        CHECK(result->raw_get<Function>()->call({})->raw_get<String>()
              == "2024-01-15T10:30:00+05:30");

        auto* db = dynamic_cast<const Data_Builtin<tomlpp::date_time>*>(
            result->raw_get<Function>().get());
        REQUIRE(db != nullptr);
        CHECK(db->data().offset.value().minutes == 330);
    }

    SECTION("invalid input")
    {
        CHECK_THROWS_WITH(datetime_fn->call({Value::create("not-a-datetime"s)}),
                          ContainsSubstring("datetime"));
    }

    SECTION("rejects plain integer")
    {
        CHECK_THROWS_WITH(datetime_fn->call({Value::create("42"s)}),
                          ContainsSubstring("not a datetime"));
    }
}

TEST_CASE("toml.special_float: constructor")
{
    auto mod = toml_module();
    auto special_float_fn = lookup(mod, "special_float");

    SECTION("nan")
    {
        auto result = special_float_fn->call({Value::create("nan"s)});
        REQUIRE(result->is<Function>());
        CHECK(result->raw_get<Function>()->call({})->raw_get<String>() == "nan");

        auto* db = dynamic_cast<const Data_Builtin<double>*>(
            result->raw_get<Function>().get());
        REQUIRE(db != nullptr);
        CHECK(std::isnan(db->data()));
    }

    SECTION("inf")
    {
        auto result = special_float_fn->call({Value::create("inf"s)});
        CHECK(result->raw_get<Function>()->call({})->raw_get<String>() == "inf");

        auto* db = dynamic_cast<const Data_Builtin<double>*>(
            result->raw_get<Function>().get());
        REQUIRE(db != nullptr);
        CHECK(std::isinf(db->data()));
        CHECK(db->data() > 0);
    }

    SECTION("-inf")
    {
        auto result = special_float_fn->call({Value::create("-inf"s)});
        CHECK(result->raw_get<Function>()->call({})->raw_get<String>()
              == "-inf");

        auto* db = dynamic_cast<const Data_Builtin<double>*>(
            result->raw_get<Function>().get());
        REQUIRE(db != nullptr);
        CHECK(std::isinf(db->data()));
        CHECK(db->data() < 0);
    }

    SECTION("invalid string")
    {
        CHECK_THROWS_WITH(special_float_fn->call({Value::create("NaN"s)}),
                          ContainsSubstring("nan"));
    }
}
