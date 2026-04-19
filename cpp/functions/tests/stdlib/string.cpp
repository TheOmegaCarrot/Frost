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
        auto result =
            fn->call({Value::create("hello world"s), Value::create("hello"s)});
        CHECK(result->raw_get<Int>() == 0);
    }

    SECTION("found in middle")
    {
        auto result =
            fn->call({Value::create("hello world"s), Value::create("world"s)});
        CHECK(result->raw_get<Int>() == 6);
    }

    SECTION("not found returns null")
    {
        auto result =
            fn->call({Value::create("hello"s), Value::create("xyz"s)});
        CHECK(result == Value::null());
    }

    SECTION("empty substring matches at 0")
    {
        auto result = fn->call({Value::create("hello"s), Value::create(""s)});
        CHECK(result->raw_get<Int>() == 0);
    }

    SECTION("finds first occurrence")
    {
        auto result =
            fn->call({Value::create("abcabc"s), Value::create("bc"s)});
        CHECK(result->raw_get<Int>() == 1);
    }

    SECTION("arity")
    {
        CHECK_THROWS_WITH(fn->call({}), ContainsSubstring("insufficient"));
        CHECK_THROWS_WITH(fn->call({Value::create("a"s)}),
                          ContainsSubstring("insufficient"));
        CHECK_THROWS_WITH(fn->call({Value::create("a"s), Value::create("b"s),
                                    Value::create("c"s)}),
                          ContainsSubstring("too many"));
    }

    SECTION("type constraints")
    {
        CHECK_THROWS_WITH(fn->call({Value::create(42_f), Value::create("x"s)}),
                          ContainsSubstring("String"));
        CHECK_THROWS_WITH(fn->call({Value::create("x"s), Value::create(42_f)}),
                          ContainsSubstring("String"));
    }
}

TEST_CASE("string.last_index_of")
{
    auto mod = string_module();
    auto fn = lookup(mod, "last_index_of");

    SECTION("finds last occurrence")
    {
        auto result =
            fn->call({Value::create("abcabc"s), Value::create("bc"s)});
        CHECK(result->raw_get<Int>() == 4);
    }

    SECTION("single occurrence same as index_of")
    {
        auto result =
            fn->call({Value::create("hello world"s), Value::create("world"s)});
        CHECK(result->raw_get<Int>() == 6);
    }

    SECTION("not found returns null")
    {
        auto result =
            fn->call({Value::create("hello"s), Value::create("xyz"s)});
        CHECK(result == Value::null());
    }

    SECTION("arity")
    {
        CHECK_THROWS_WITH(fn->call({}), ContainsSubstring("insufficient"));
        CHECK_THROWS_WITH(fn->call({Value::create("a"s)}),
                          ContainsSubstring("insufficient"));
        CHECK_THROWS_WITH(fn->call({Value::create("a"s), Value::create("b"s),
                                    Value::create("c"s)}),
                          ContainsSubstring("too many"));
    }

    SECTION("type constraints")
    {
        CHECK_THROWS_WITH(fn->call({Value::create(42_f), Value::create("x"s)}),
                          ContainsSubstring("String"));
        CHECK_THROWS_WITH(fn->call({Value::create("x"s), Value::create(42_f)}),
                          ContainsSubstring("String"));
    }
}

TEST_CASE("string.count")
{
    auto mod = string_module();
    auto fn = lookup(mod, "count");

    SECTION("multiple occurrences")
    {
        auto result =
            fn->call({Value::create("banana"s), Value::create("an"s)});
        CHECK(result->raw_get<Int>() == 2);
    }

    SECTION("no occurrences")
    {
        auto result =
            fn->call({Value::create("hello"s), Value::create("xyz"s)});
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
        CHECK_THROWS_WITH(fn->call({Value::create("a"s)}),
                          ContainsSubstring("insufficient"));
        CHECK_THROWS_WITH(fn->call({Value::create("a"s), Value::create("b"s),
                                    Value::create("c"s)}),
                          ContainsSubstring("too many"));
    }

    SECTION("type constraints")
    {
        CHECK_THROWS_WITH(fn->call({Value::create(42_f), Value::create("x"s)}),
                          ContainsSubstring("String"));
        CHECK_THROWS_WITH(fn->call({Value::create("x"s), Value::create(42_f)}),
                          ContainsSubstring("String"));
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
        CHECK_THROWS_WITH(fn->call({Value::create("a"s), Value::create("b"s)}),
                          ContainsSubstring("too many"));
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

TEST_CASE("string.is_*: arity and type constraints")
{
    auto mod = string_module();

    for (const auto& name : {"is_empty"s, "is_ascii"s, "is_digit"s, "is_alpha"s,
                             "is_alphanumeric"s, "is_whitespace"s,
                             "is_uppercase"s, "is_lowercase"s})
    {
        DYNAMIC_SECTION(name)
        {
            auto fn = lookup(mod, name);

            CHECK_THROWS_WITH(fn->call({}), ContainsSubstring("insufficient"));
            CHECK_THROWS_WITH(
                fn->call({Value::create("a"s), Value::create("b"s)}),
                ContainsSubstring("too many"));
            CHECK_THROWS_WITH(fn->call({Value::create(42_f)}),
                              ContainsSubstring("String"));
        }
    }
}

TEST_CASE("string.pad_left")
{
    auto mod = string_module();
    auto fn = lookup(mod, "pad_left");

    SECTION("pads with spaces by default")
    {
        auto result = fn->call({Value::create("hi"s), Value::create(5_f)});
        CHECK(result->raw_get<String>() == "   hi");
    }

    SECTION("custom fill character")
    {
        auto result = fn->call(
            {Value::create("42"s), Value::create(5_f), Value::create("0"s)});
        CHECK(result->raw_get<String>() == "00042");
    }

    SECTION("string already at width")
    {
        auto result = fn->call({Value::create("hello"s), Value::create(5_f)});
        CHECK(result->raw_get<String>() == "hello");
    }

    SECTION("string longer than width")
    {
        auto result = fn->call({Value::create("hello"s), Value::create(3_f)});
        CHECK(result->raw_get<String>() == "hello");
    }

    SECTION("width zero")
    {
        auto result = fn->call({Value::create("hi"s), Value::create(0_f)});
        CHECK(result->raw_get<String>() == "hi");
    }

    SECTION("fill must be single byte")
    {
        CHECK_THROWS_WITH(fn->call({Value::create("x"s), Value::create(5_f),
                                    Value::create("ab"s)}),
                          ContainsSubstring("single byte"));
    }

    SECTION("arity")
    {
        CHECK_THROWS_WITH(fn->call({}), ContainsSubstring("insufficient"));
        CHECK_THROWS_WITH(fn->call({Value::create("a"s)}),
                          ContainsSubstring("insufficient"));
        CHECK_THROWS_WITH(fn->call({Value::create("a"s), Value::create(5_f),
                                    Value::create("x"s), Value::create("y"s)}),
                          ContainsSubstring("too many"));
    }

    SECTION("type constraints")
    {
        CHECK_THROWS_WITH(fn->call({Value::create(42_f), Value::create(5_f)}),
                          ContainsSubstring("String"));
        CHECK_THROWS_WITH(fn->call({Value::create("x"s), Value::create("y"s)}),
                          ContainsSubstring("Int"));
    }
}

TEST_CASE("string.pad_right")
{
    auto mod = string_module();
    auto fn = lookup(mod, "pad_right");

    SECTION("pads with spaces by default")
    {
        auto result = fn->call({Value::create("hi"s), Value::create(5_f)});
        CHECK(result->raw_get<String>() == "hi   ");
    }

    SECTION("custom fill character")
    {
        auto result = fn->call(
            {Value::create("hi"s), Value::create(5_f), Value::create("."s)});
        CHECK(result->raw_get<String>() == "hi...");
    }

    SECTION("string already at width")
    {
        auto result = fn->call({Value::create("hello"s), Value::create(5_f)});
        CHECK(result->raw_get<String>() == "hello");
    }

    SECTION("string longer than width")
    {
        auto result = fn->call({Value::create("hello"s), Value::create(3_f)});
        CHECK(result->raw_get<String>() == "hello");
    }

    SECTION("arity")
    {
        CHECK_THROWS_WITH(fn->call({}), ContainsSubstring("insufficient"));
        CHECK_THROWS_WITH(fn->call({Value::create("a"s)}),
                          ContainsSubstring("insufficient"));
        CHECK_THROWS_WITH(fn->call({Value::create("a"s), Value::create(5_f),
                                    Value::create("x"s), Value::create("y"s)}),
                          ContainsSubstring("too many"));
    }

    SECTION("type constraints")
    {
        CHECK_THROWS_WITH(fn->call({Value::create(42_f), Value::create(5_f)}),
                          ContainsSubstring("String"));
        CHECK_THROWS_WITH(fn->call({Value::create("x"s), Value::create("y"s)}),
                          ContainsSubstring("Int"));
    }
}

TEST_CASE("string.center")
{
    auto mod = string_module();
    auto fn = lookup(mod, "center");

    SECTION("centers with spaces by default")
    {
        auto result = fn->call({Value::create("hi"s), Value::create(6_f)});
        CHECK(result->raw_get<String>() == "  hi  ");
    }

    SECTION("odd padding favors right")
    {
        auto result = fn->call({Value::create("hi"s), Value::create(5_f)});
        CHECK(result->raw_get<String>() == " hi  ");
    }

    SECTION("custom fill character")
    {
        auto result = fn->call(
            {Value::create("hi"s), Value::create(6_f), Value::create("-"s)});
        CHECK(result->raw_get<String>() == "--hi--");
    }

    SECTION("string already at width")
    {
        auto result = fn->call({Value::create("hello"s), Value::create(5_f)});
        CHECK(result->raw_get<String>() == "hello");
    }

    SECTION("string longer than width")
    {
        auto result = fn->call({Value::create("hello"s), Value::create(3_f)});
        CHECK(result->raw_get<String>() == "hello");
    }

    SECTION("fill must be single byte")
    {
        CHECK_THROWS_WITH(fn->call({Value::create("x"s), Value::create(5_f),
                                    Value::create("ab"s)}),
                          ContainsSubstring("single byte"));
    }

    SECTION("arity")
    {
        CHECK_THROWS_WITH(fn->call({}), ContainsSubstring("insufficient"));
        CHECK_THROWS_WITH(fn->call({Value::create("a"s)}),
                          ContainsSubstring("insufficient"));
        CHECK_THROWS_WITH(fn->call({Value::create("a"s), Value::create(5_f),
                                    Value::create("x"s), Value::create("y"s)}),
                          ContainsSubstring("too many"));
    }

    SECTION("type constraints")
    {
        CHECK_THROWS_WITH(fn->call({Value::create(42_f), Value::create(5_f)}),
                          ContainsSubstring("String"));
        CHECK_THROWS_WITH(fn->call({Value::create("x"s), Value::create("y"s)}),
                          ContainsSubstring("Int"));
    }
}

TEST_CASE("string.repeat")
{
    auto mod = string_module();
    auto fn = lookup(mod, "repeat");

    SECTION("basic repeat")
    {
        auto result = fn->call({Value::create("ab"s), Value::create(3_f)});
        CHECK(result->raw_get<String>() == "ababab");
    }

    SECTION("repeat once")
    {
        auto result = fn->call({Value::create("hi"s), Value::create(1_f)});
        CHECK(result->raw_get<String>() == "hi");
    }

    SECTION("repeat zero times")
    {
        auto result = fn->call({Value::create("hi"s), Value::create(0_f)});
        CHECK(result->raw_get<String>() == "");
    }

    SECTION("empty string repeated")
    {
        auto result = fn->call({Value::create(""s), Value::create(5_f)});
        CHECK(result->raw_get<String>() == "");
    }

    SECTION("negative count rejected")
    {
        CHECK_THROWS_WITH(fn->call({Value::create("x"s), Value::create(-1_f)}),
                          ContainsSubstring("negative"));
    }

    SECTION("arity")
    {
        CHECK_THROWS_WITH(fn->call({}), ContainsSubstring("insufficient"));
        CHECK_THROWS_WITH(fn->call({Value::create("a"s)}),
                          ContainsSubstring("insufficient"));
        CHECK_THROWS_WITH(fn->call({Value::create("a"s), Value::create(3_f),
                                    Value::create("x"s)}),
                          ContainsSubstring("too many"));
    }

    SECTION("type constraints")
    {
        CHECK_THROWS_WITH(fn->call({Value::create(42_f), Value::create(3_f)}),
                          ContainsSubstring("String"));
        CHECK_THROWS_WITH(fn->call({Value::create("x"s), Value::create("y"s)}),
                          ContainsSubstring("Int"));
    }
}
