#include <catch2/catch_all.hpp>

#include <frost/value.hpp>

using namespace std::literals;
using namespace frst::literals;
using frst::Value, frst::Value_Ptr;

TEST_CASE("to_string")
{
    auto Null = Value::create();
    auto Int = Value::create(42_f);
    auto Float = Value::create(3.14);
    auto Bool = Value::create(true);
    auto String = Value::create("Hello!"s);
    auto Array = Value::create(frst::Array{
        Value::create(42_f),
        Value::create("hello"s),
    });
    auto Map = Value::create(frst::Map{
        {Value::create("key1"s), Value::create(42_f)},
        {Value::create(true), Value::create("value2"s)},
    });

    CHECK(Null->to_internal_string() == "null");
    CHECK(Int->to_internal_string() == "42");
    CHECK(Float->to_internal_string().starts_with("3.14"));
    CHECK(Bool->to_internal_string() == "true");
    CHECK(String->to_internal_string() == "Hello!");
    CHECK(Array->to_internal_string() == R"([ 42, "hello" ])");
    CHECK(Map->to_internal_string() == R"({ [true]: "value2", ["key1"]: 42 })");

    auto Nested = Value::create(frst::Array{
        Value::create(42_f),
        Array,
    });

    CHECK(Nested->to_internal_string() == R"([ 42, [ 42, "hello" ] ])");
}
