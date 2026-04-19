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

TEST_CASE("string.is_empty")
{
    auto mod = string_module();
    auto fn = lookup(mod, "is_empty");

    CHECK(fn->call({Value::create(""s)})->raw_get<Bool>() == true);
    CHECK(fn->call({Value::create("x"s)})->raw_get<Bool>() == false);
    CHECK(fn->call({Value::create(" "s)})->raw_get<Bool>() == false);
}

TEST_CASE("string.is_ascii")
{
    auto mod = string_module();
    auto fn = lookup(mod, "is_ascii");

    CHECK(fn->call({Value::create("hello"s)})->raw_get<Bool>() == true);
    CHECK(fn->call({Value::create(""s)})->raw_get<Bool>() == true);
    CHECK(fn->call({Value::create("abc123"s)})->raw_get<Bool>() == true);
    CHECK(fn->call({Value::create("\t\n"s)})->raw_get<Bool>() == true);

    // non-ASCII byte
    CHECK(fn->call({Value::create(std::string{"\x80"})})->raw_get<Bool>()
          == false);
    CHECK(fn->call({Value::create(std::string{"caf\xc3\xa9"})})->raw_get<Bool>()
          == false);
}

TEST_CASE("string.is_digit")
{
    auto mod = string_module();
    auto fn = lookup(mod, "is_digit");

    CHECK(fn->call({Value::create("12345"s)})->raw_get<Bool>() == true);
    CHECK(fn->call({Value::create("0"s)})->raw_get<Bool>() == true);
    CHECK(fn->call({Value::create(""s)})->raw_get<Bool>() == true);
    CHECK(fn->call({Value::create("12a"s)})->raw_get<Bool>() == false);
    CHECK(fn->call({Value::create("abc"s)})->raw_get<Bool>() == false);
    CHECK(fn->call({Value::create(" "s)})->raw_get<Bool>() == false);
}

TEST_CASE("string.is_alpha")
{
    auto mod = string_module();
    auto fn = lookup(mod, "is_alpha");

    CHECK(fn->call({Value::create("hello"s)})->raw_get<Bool>() == true);
    CHECK(fn->call({Value::create(""s)})->raw_get<Bool>() == true);
    CHECK(fn->call({Value::create("abc123"s)})->raw_get<Bool>() == false);
    CHECK(fn->call({Value::create("hello world"s)})->raw_get<Bool>() == false);
}

TEST_CASE("string.is_alphanumeric")
{
    auto mod = string_module();
    auto fn = lookup(mod, "is_alphanumeric");

    CHECK(fn->call({Value::create("abc123"s)})->raw_get<Bool>() == true);
    CHECK(fn->call({Value::create("hello"s)})->raw_get<Bool>() == true);
    CHECK(fn->call({Value::create("42"s)})->raw_get<Bool>() == true);
    CHECK(fn->call({Value::create(""s)})->raw_get<Bool>() == true);
    CHECK(fn->call({Value::create("hello world"s)})->raw_get<Bool>() == false);
    CHECK(fn->call({Value::create("a-b"s)})->raw_get<Bool>() == false);
}

TEST_CASE("string.is_whitespace")
{
    auto mod = string_module();
    auto fn = lookup(mod, "is_whitespace");

    CHECK(fn->call({Value::create(" "s)})->raw_get<Bool>() == true);
    CHECK(fn->call({Value::create("\t\n\r "s)})->raw_get<Bool>() == true);
    CHECK(fn->call({Value::create(""s)})->raw_get<Bool>() == true);
    CHECK(fn->call({Value::create("hello"s)})->raw_get<Bool>() == false);
    CHECK(fn->call({Value::create(" a "s)})->raw_get<Bool>() == false);
}

TEST_CASE("string.is_uppercase")
{
    auto mod = string_module();
    auto fn = lookup(mod, "is_uppercase");

    CHECK(fn->call({Value::create("HELLO"s)})->raw_get<Bool>() == true);
    CHECK(fn->call({Value::create("HELLO123"s)})->raw_get<Bool>() == true);
    CHECK(fn->call({Value::create("123"s)})->raw_get<Bool>() == true);
    CHECK(fn->call({Value::create(""s)})->raw_get<Bool>() == true);
    CHECK(fn->call({Value::create("Hello"s)})->raw_get<Bool>() == false);
    CHECK(fn->call({Value::create("hello"s)})->raw_get<Bool>() == false);
}

TEST_CASE("string.is_lowercase")
{
    auto mod = string_module();
    auto fn = lookup(mod, "is_lowercase");

    CHECK(fn->call({Value::create("hello"s)})->raw_get<Bool>() == true);
    CHECK(fn->call({Value::create("hello123"s)})->raw_get<Bool>() == true);
    CHECK(fn->call({Value::create("123"s)})->raw_get<Bool>() == true);
    CHECK(fn->call({Value::create(""s)})->raw_get<Bool>() == true);
    CHECK(fn->call({Value::create("Hello"s)})->raw_get<Bool>() == false);
    CHECK(fn->call({Value::create("HELLO"s)})->raw_get<Bool>() == false);
}
