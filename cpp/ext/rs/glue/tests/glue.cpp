#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/value.hpp>

#include "frost-glue.hpp"

#include <rust/cxx.h>

using namespace frst;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;

// Forward-declare the cxx-generated Rust test helpers
namespace frst::rs::test
{
Value_Ptr rt_identity(::rust::Slice<const Value_Ptr> args);
Value_Ptr rt_type_name(::rust::Slice<const Value_Ptr> args);
Value_Ptr rt_round_trip(::rust::Slice<const Value_Ptr> args);
Value_Ptr rt_array_sum(::rust::Slice<const Value_Ptr> args);
Value_Ptr rt_concat_all(::rust::Slice<const Value_Ptr> args);
} // namespace frst::rs::test

namespace
{

Value_Ptr call(auto fn, std::initializer_list<Value_Ptr> args)
{
    auto vec = std::vector<Value_Ptr>(args);
    return fn({vec.data(), vec.size()});
}

} // namespace

TEST_CASE("identity: values survive C++ -> Rust -> C++ unchanged")
{
    SECTION("null")
    {
        auto result = call(frst::rs::test::rt_identity, {Value::null()});
        CHECK(result->is<Null>());
    }

    SECTION("int")
    {
        auto result = call(frst::rs::test::rt_identity, {Value::create(42_f)});
        REQUIRE(result->is<Int>());
        CHECK(result->raw_get<Int>() == 42);
    }

    SECTION("float")
    {
        auto result =
            call(frst::rs::test::rt_identity, {Value::create(Float{3.14})});
        REQUIRE(result->is<Float>());
        CHECK(result->raw_get<Float>() == Catch::Approx(3.14));
    }

    SECTION("bool")
    {
        auto result =
            call(frst::rs::test::rt_identity, {Value::create(Bool{true})});
        REQUIRE(result->is<Bool>());
        CHECK(result->raw_get<Bool>() == true);
    }

    SECTION("string")
    {
        auto result =
            call(frst::rs::test::rt_identity, {Value::create("hello"s)});
        REQUIRE(result->is<String>());
        CHECK(result->raw_get<String>() == "hello");
    }

    SECTION("array")
    {
        auto arr = Value::create(
            Array{Value::create(1_f), Value::create(2_f), Value::create(3_f)});
        auto result = call(frst::rs::test::rt_identity, {arr});
        REQUIRE(result->is<Array>());
        CHECK(result->raw_get<Array>().size() == 3);
    }
}

TEST_CASE("type_name: Rust can inspect Value types")
{
    auto check = [](Value_Ptr val, const std::string& expected) {
        auto result = call(frst::rs::test::rt_type_name, {val});
        REQUIRE(result->is<String>());
        CHECK(result->raw_get<String>() == expected);
    };

    check(Value::null(), "Null");
    check(Value::create(42_f), "Int");
    check(Value::create(Float{1.0}), "Float");
    check(Value::create(Bool{false}), "Bool");
    check(Value::create("hi"s), "String");
    check(Value::create(Array{}), "Array");
    check(Value::create(Value::trusted, Map{}), "Map");
}

TEST_CASE("round_trip: Rust extracts and reconstructs primitives")
{
    SECTION("int")
    {
        auto result =
            call(frst::rs::test::rt_round_trip, {Value::create(99_f)});
        REQUIRE(result->is<Int>());
        CHECK(result->raw_get<Int>() == 99);
    }

    SECTION("float")
    {
        auto result =
            call(frst::rs::test::rt_round_trip, {Value::create(Float{2.718})});
        REQUIRE(result->is<Float>());
        CHECK(result->raw_get<Float>() == Catch::Approx(2.718));
    }

    SECTION("bool true")
    {
        auto result =
            call(frst::rs::test::rt_round_trip, {Value::create(Bool{true})});
        REQUIRE(result->is<Bool>());
        CHECK(result->raw_get<Bool>() == true);
    }

    SECTION("bool false")
    {
        auto result =
            call(frst::rs::test::rt_round_trip, {Value::create(Bool{false})});
        REQUIRE(result->is<Bool>());
        CHECK(result->raw_get<Bool>() == false);
    }

    SECTION("string")
    {
        auto result = call(frst::rs::test::rt_round_trip,
                           {Value::create("round trip"s)});
        REQUIRE(result->is<String>());
        CHECK(result->raw_get<String>() == "round trip");
    }

    SECTION("null")
    {
        auto result = call(frst::rs::test::rt_round_trip, {Value::null()});
        CHECK(result->is<Null>());
    }

    SECTION("empty string")
    {
        auto result =
            call(frst::rs::test::rt_round_trip, {Value::create(""s)});
        REQUIRE(result->is<String>());
        CHECK(result->raw_get<String>().empty());
    }
}

TEST_CASE("array_sum: Rust reads array elements via zero-copy slice")
{
    SECTION("simple sum")
    {
        auto arr = Value::create(Array{
            Value::create(10_f), Value::create(20_f), Value::create(30_f)});
        auto result = call(frst::rs::test::rt_array_sum, {arr});
        REQUIRE(result->is<Int>());
        CHECK(result->raw_get<Int>() == 60);
    }

    SECTION("single element")
    {
        auto arr = Value::create(Array{Value::create(7_f)});
        auto result = call(frst::rs::test::rt_array_sum, {arr});
        REQUIRE(result->is<Int>());
        CHECK(result->raw_get<Int>() == 7);
    }

    SECTION("empty array")
    {
        auto arr = Value::create(Array{});
        auto result = call(frst::rs::test::rt_array_sum, {arr});
        REQUIRE(result->is<Int>());
        CHECK(result->raw_get<Int>() == 0);
    }
}

TEST_CASE("concat_all: variadic args via slice")
{
    SECTION("two strings")
    {
        auto result = call(frst::rs::test::rt_concat_all,
                           {Value::create("hello"s), Value::create(", world"s)});
        REQUIRE(result->is<String>());
        CHECK(result->raw_get<String>() == "hello, world");
    }

    SECTION("many strings")
    {
        auto result = call(frst::rs::test::rt_concat_all,
                           {Value::create("a"s), Value::create("b"s),
                            Value::create("c"s), Value::create("d"s)});
        REQUIRE(result->is<String>());
        CHECK(result->raw_get<String>() == "abcd");
    }

    SECTION("single string")
    {
        auto result =
            call(frst::rs::test::rt_concat_all, {Value::create("solo"s)});
        REQUIRE(result->is<String>());
        CHECK(result->raw_get<String>() == "solo");
    }

    SECTION("empty args returns empty string")
    {
        auto result = call(frst::rs::test::rt_concat_all, {});
        REQUIRE(result->is<String>());
        CHECK(result->raw_get<String>().empty());
    }
}

TEST_CASE("error propagation: Rust Err becomes rust::Error")
{
    SECTION("identity with no args")
    {
        CHECK_THROWS_WITH(call(frst::rs::test::rt_identity, {}),
                          ContainsSubstring("expected at least 1 argument"));
    }

    SECTION("array_sum with non-array")
    {
        CHECK_THROWS_WITH(
            call(frst::rs::test::rt_array_sum, {Value::create(42_f)}),
            ContainsSubstring("expected an Array"));
    }

    SECTION("concat_all with non-string")
    {
        CHECK_THROWS_WITH(
            call(frst::rs::test::rt_concat_all, {Value::create(42_f)}),
            ContainsSubstring("expected String"));
    }
}
