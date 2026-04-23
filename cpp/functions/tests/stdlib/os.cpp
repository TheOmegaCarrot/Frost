#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_exception.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/builtin.hpp>
#include <frost/stdlib.hpp>
#include <frost/value.hpp>

using namespace frst;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;
using Catch::Matchers::MessageMatches;

namespace
{

Map os_module()
{
    Stdlib_Registry_Builder builder;
    register_module_os(builder);
    auto registry = std::move(builder).build();
    auto module = registry.lookup_module("std.os");
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

TEST_CASE("std.os")
{
    auto mod = os_module();

    SECTION("Registered")
    {
        CHECK(mod.size() == 7);
        lookup(mod, "getenv");
        lookup(mod, "setenv");
        lookup(mod, "exit");
        lookup(mod, "sleep");
        lookup(mod, "run");
        lookup(mod, "pid");
        lookup(mod, "hostname");
    }

    SECTION("getenv")
    {
        auto getenv = lookup(mod, "getenv");

        SECTION("Arity")
        {
            CHECK_THROWS_MATCHES(
                getenv->call({}), Frost_User_Error,
                MessageMatches(ContainsSubstring("insufficient arguments")
                               && ContainsSubstring("requires at least 1")));
        }

        SECTION("Type error")
        {
            CHECK_THROWS_MATCHES(getenv->call({Value::create(1_f)}),
                                 Frost_User_Error,
                                 MessageMatches(ContainsSubstring("os.getenv")
                                                && ContainsSubstring("String")
                                                && ContainsSubstring("Int")));
        }

        SECTION("Returns null for unset variable")
        {
            auto result = getenv->call(
                {Value::create("FROST_DEFINITELY_NOT_SET_12345"s)});
            CHECK(result->is<Null>());
        }

        SECTION("Returns string for set variable")
        {
            auto result = getenv->call({Value::create("PATH"s)});
            REQUIRE(result->is<String>());
            CHECK(not result->raw_get<String>().empty());
        }
    }

    SECTION("setenv")
    {
        auto setenv_fn = lookup(mod, "setenv");
        auto getenv_fn = lookup(mod, "getenv");

        SECTION("Arity")
        {
            CHECK_THROWS_MATCHES(
                setenv_fn->call({}), Frost_User_Error,
                MessageMatches(ContainsSubstring("insufficient arguments")
                               && ContainsSubstring("requires at least 2")));
            CHECK_THROWS_MATCHES(
                setenv_fn->call({Value::create("X"s)}), Frost_User_Error,
                MessageMatches(ContainsSubstring("insufficient arguments")));
        }

        SECTION("Type errors")
        {
            CHECK_THROWS_MATCHES(
                setenv_fn->call({Value::create(1_f), Value::create("v"s)}),
                Frost_User_Error,
                MessageMatches(ContainsSubstring("os.setenv")
                               && ContainsSubstring("variable")
                               && ContainsSubstring("got Int")));
            CHECK_THROWS_MATCHES(
                setenv_fn->call({Value::create("k"s), Value::create(1_f)}),
                Frost_User_Error,
                MessageMatches(ContainsSubstring("os.setenv")
                               && ContainsSubstring("value")
                               && ContainsSubstring("got Int")));
        }

        SECTION("Sets and overwrites environment variable")
        {
            auto name = "FROST_TEST_SETENV_42"s;
            setenv_fn->call(
                {Value::create(String{name}), Value::create("first"s)});
            auto result = getenv_fn->call({Value::create(String{name})});
            REQUIRE(result->is<String>());
            CHECK(result->raw_get<String>() == "first");

            setenv_fn->call(
                {Value::create(String{name}), Value::create("second"s)});
            auto result2 = getenv_fn->call({Value::create(String{name})});
            REQUIRE(result2->is<String>());
            CHECK(result2->raw_get<String>() == "second");
        }

        SECTION("Returns null")
        {
            auto result = setenv_fn->call(
                {Value::create("FROST_TEST_SETENV_NULL"s),
                 Value::create("x"s)});
            CHECK(result->is<Null>());
        }
    }

    SECTION("sleep")
    {
        auto sleep = lookup(mod, "sleep");

        SECTION("Arity")
        {
            CHECK_THROWS_MATCHES(
                sleep->call({}), Frost_User_Error,
                MessageMatches(ContainsSubstring("insufficient arguments")
                               && ContainsSubstring("requires at least 1")));
        }

        SECTION("Type error")
        {
            CHECK_THROWS_MATCHES(
                sleep->call({Value::create("10"s)}), Frost_User_Error,
                MessageMatches(ContainsSubstring("os.sleep")
                               && ContainsSubstring("Int")
                               && ContainsSubstring("String")));
        }

        SECTION("Returns null")
        {
            auto result = sleep->call({Value::create(0_f)});
            CHECK(result->is<Null>());
        }
    }

    SECTION("exit")
    {
        auto exit = lookup(mod, "exit");

        SECTION("Arity")
        {
            CHECK_THROWS_MATCHES(
                exit->call({}), Frost_User_Error,
                MessageMatches(ContainsSubstring("insufficient arguments")
                               && ContainsSubstring("requires at least 1")));
        }

        SECTION("Type error")
        {
            CHECK_THROWS_MATCHES(
                exit->call({Value::create("0"s)}), Frost_User_Error,
                MessageMatches(ContainsSubstring("os.exit")
                               && ContainsSubstring("Int")
                               && ContainsSubstring("String")));
        }
    }

    SECTION("pid")
    {
        auto pid = lookup(mod, "pid");

        SECTION("returns a positive Int")
        {
            auto result = pid->call({});
            REQUIRE(result->is<Int>());
            CHECK(result->raw_get<Int>() > 0);
        }

        SECTION("arity")
        {
            CHECK_THROWS_MATCHES(pid->call({Value::create(1_f)}),
                                 Frost_User_Error,
                                 MessageMatches(ContainsSubstring("too many")));
        }
    }

    SECTION("hostname")
    {
        auto hostname = lookup(mod, "hostname");

        SECTION("returns a non-empty String")
        {
            auto result = hostname->call({});
            REQUIRE(result->is<String>());
            CHECK(not result->raw_get<String>().empty());
        }

        SECTION("arity")
        {
            CHECK_THROWS_MATCHES(hostname->call({Value::create(1_f)}),
                                 Frost_User_Error,
                                 MessageMatches(ContainsSubstring("too many")));
        }
    }
}
