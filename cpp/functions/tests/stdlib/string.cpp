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

Map string_module()
{
    Stdlib_Registry_Builder builder;
    register_module_string(builder);
    auto registry = std::move(builder).build();
    auto module = registry.lookup_module("std.string");
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

TEST_CASE("string.index_of")
{
    auto mod = string_module();
    auto fn = lookup(mod, "index_of");

    SECTION("found at start")
    {
        auto result = fn->call({Value::create("hello world"s), Value::create("hello"s)});
        CHECK(result->raw_get<Int>() == 0);
    }

    SECTION("found in middle")
    {
        auto result = fn->call({Value::create("hello world"s), Value::create("world"s)});
        CHECK(result->raw_get<Int>() == 6);
    }

    SECTION("not found returns null")
    {
        auto result = fn->call({Value::create("hello"s), Value::create("xyz"s)});
        CHECK(result == Value::null());
    }

    SECTION("empty substring matches at 0")
    {
        auto result = fn->call({Value::create("hello"s), Value::create(""s)});
        CHECK(result->raw_get<Int>() == 0);
    }

    SECTION("finds first occurrence")
    {
        auto result = fn->call({Value::create("abcabc"s), Value::create("bc"s)});
        CHECK(result->raw_get<Int>() == 1);
    }

    SECTION("arity")
    {
        CHECK_THROWS_WITH(fn->call({}), ContainsSubstring("insufficient"));
        CHECK_THROWS_WITH(fn->call({Value::create("a"s)}),
                          ContainsSubstring("insufficient"));
    }

    SECTION("type constraint")
    {
        CHECK_THROWS_WITH(fn->call({Value::create(42_f), Value::create("x"s)}),
                          ContainsSubstring("String"));
    }
}

TEST_CASE("string.last_index_of")
{
    auto mod = string_module();
    auto fn = lookup(mod, "last_index_of");

    SECTION("finds last occurrence")
    {
        auto result = fn->call({Value::create("abcabc"s), Value::create("bc"s)});
        CHECK(result->raw_get<Int>() == 4);
    }

    SECTION("single occurrence same as index_of")
    {
        auto result = fn->call({Value::create("hello world"s), Value::create("world"s)});
        CHECK(result->raw_get<Int>() == 6);
    }

    SECTION("not found returns null")
    {
        auto result = fn->call({Value::create("hello"s), Value::create("xyz"s)});
        CHECK(result == Value::null());
    }

    SECTION("arity")
    {
        CHECK_THROWS_WITH(fn->call({}), ContainsSubstring("insufficient"));
    }
}

TEST_CASE("string.count")
{
    auto mod = string_module();
    auto fn = lookup(mod, "count");

    SECTION("multiple occurrences")
    {
        auto result = fn->call({Value::create("banana"s), Value::create("an"s)});
        CHECK(result->raw_get<Int>() == 2);
    }

    SECTION("no occurrences")
    {
        auto result = fn->call({Value::create("hello"s), Value::create("xyz"s)});
        CHECK(result->raw_get<Int>() == 0);
    }

    SECTION("non-overlapping")
    {
        auto result = fn->call({Value::create("aaa"s), Value::create("aa"s)});
        CHECK(result->raw_get<Int>() == 1);
    }

    SECTION("single character")
    {
        auto result = fn->call({Value::create("hello"s), Value::create("l"s)});
        CHECK(result->raw_get<Int>() == 2);
    }

    SECTION("empty substring rejected")
    {
        CHECK_THROWS_WITH(
            fn->call({Value::create("hello"s), Value::create(""s)}),
            ContainsSubstring("empty"));
    }

    SECTION("arity")
    {
        CHECK_THROWS_WITH(fn->call({}), ContainsSubstring("insufficient"));
    }
}

TEST_CASE("string.chars")
{
    auto mod = string_module();
    auto fn = lookup(mod, "chars");

    SECTION("splits into single characters")
    {
        auto result = fn->call({Value::create("abc"s)});
        REQUIRE(result->is<Array>());
        const auto& arr = result->raw_get<Array>();
        REQUIRE(arr.size() == 3);
        CHECK(arr[0]->raw_get<String>() == "a");
        CHECK(arr[1]->raw_get<String>() == "b");
        CHECK(arr[2]->raw_get<String>() == "c");
    }

    SECTION("empty string gives empty array")
    {
        auto result = fn->call({Value::create(""s)});
        REQUIRE(result->is<Array>());
        CHECK(result->raw_get<Array>().empty());
    }

    SECTION("single character")
    {
        auto result = fn->call({Value::create("x"s)});
        REQUIRE(result->raw_get<Array>().size() == 1);
        CHECK(result->raw_get<Array>()[0]->raw_get<String>() == "x");
    }

    SECTION("arity")
    {
        CHECK_THROWS_WITH(fn->call({}), ContainsSubstring("insufficient"));
    }

    SECTION("type constraint")
    {
        CHECK_THROWS_WITH(fn->call({Value::create(42_f)}),
                          ContainsSubstring("String"));
    }
}
