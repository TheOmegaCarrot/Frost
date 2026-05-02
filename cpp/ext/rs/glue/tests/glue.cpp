#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/builtin.hpp>
#include <frost/value.hpp>

#include "frost-glue.hpp"

#include <rust/cxx.h>

using namespace frst;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;

namespace frst::rs::test
{
Value_Ptr rt_identity(::rust::Slice<const Value_Ptr> args);
Value_Ptr rt_type_name(::rust::Slice<const Value_Ptr> args);
Value_Ptr rt_round_trip(::rust::Slice<const Value_Ptr> args);
Value_Ptr rt_array_sum(::rust::Slice<const Value_Ptr> args);
Value_Ptr rt_make_array(::rust::Slice<const Value_Ptr> args);
Value_Ptr rt_array_reverse(::rust::Slice<const Value_Ptr> args);
Value_Ptr rt_make_map(::rust::Slice<const Value_Ptr> args);
Value_Ptr rt_map_keys_sorted(::rust::Slice<const Value_Ptr> args);
Value_Ptr rt_map_get_by_key(::rust::Slice<const Value_Ptr> args);
Value_Ptr rt_map_entry_count(::rust::Slice<const Value_Ptr> args);
Value_Ptr rt_concat_all(::rust::Slice<const Value_Ptr> args);
Value_Ptr rt_string_byte_len(::rust::Slice<const Value_Ptr> args);
Value_Ptr rt_describe_type(::rust::Slice<const Value_Ptr> args);
Value_Ptr rt_call_callback(::rust::Slice<const Value_Ptr> args);
Value_Ptr rt_make_adder(::rust::Slice<const Value_Ptr> args);
Value_Ptr rt_make_failing_fn(::rust::Slice<const Value_Ptr> args);
} // namespace frst::rs::test

namespace
{

Value_Ptr call(auto fn, std::initializer_list<Value_Ptr> args)
{
    auto vec = std::vector<Value_Ptr>(args);
    return fn({vec.data(), vec.size()});
}

} // namespace

// ==========================================================================
// Basic value passing
// ==========================================================================

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

// ==========================================================================
// Array operations
// ==========================================================================

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

TEST_CASE("make_array: Rust constructs arrays from slices")
{
    SECTION("from multiple values")
    {
        auto result = call(frst::rs::test::rt_make_array,
                           {Value::create(1_f), Value::create(2_f),
                            Value::create(3_f)});
        REQUIRE(result->is<Array>());
        const auto& arr = result->raw_get<Array>();
        REQUIRE(arr.size() == 3);
        CHECK(arr[0]->raw_get<Int>() == 1);
        CHECK(arr[1]->raw_get<Int>() == 2);
        CHECK(arr[2]->raw_get<Int>() == 3);
    }

    SECTION("empty")
    {
        auto result = call(frst::rs::test::rt_make_array, {});
        REQUIRE(result->is<Array>());
        CHECK(result->raw_get<Array>().empty());
    }

    SECTION("mixed types")
    {
        auto result = call(frst::rs::test::rt_make_array,
                           {Value::create(42_f), Value::create("hi"s),
                            Value::null()});
        REQUIRE(result->is<Array>());
        const auto& arr = result->raw_get<Array>();
        CHECK(arr[0]->is<Int>());
        CHECK(arr[1]->is<String>());
        CHECK(arr[2]->is<Null>());
    }
}

TEST_CASE("array_reverse: Rust transforms array via slice")
{
    auto arr = Value::create(
        Array{Value::create(1_f), Value::create(2_f), Value::create(3_f)});
    auto result = call(frst::rs::test::rt_array_reverse, {arr});
    REQUIRE(result->is<Array>());
    const auto& rev = result->raw_get<Array>();
    REQUIRE(rev.size() == 3);
    CHECK(rev[0]->raw_get<Int>() == 3);
    CHECK(rev[1]->raw_get<Int>() == 2);
    CHECK(rev[2]->raw_get<Int>() == 1);
}

// ==========================================================================
// Map operations
// ==========================================================================

TEST_CASE("make_map: Rust constructs maps from parallel key/value slices")
{
    SECTION("string keys")
    {
        auto result = call(frst::rs::test::rt_make_map,
                           {Value::create("a"s), Value::create(1_f),
                            Value::create("b"s), Value::create(2_f)});
        REQUIRE(result->is<Map>());
        const auto& map = result->raw_get<Map>();
        CHECK(map.size() == 2);
        CHECK(map.at(Value::create("a"s))->raw_get<Int>() == 1);
        CHECK(map.at(Value::create("b"s))->raw_get<Int>() == 2);
    }

    SECTION("int keys")
    {
        auto result = call(frst::rs::test::rt_make_map,
                           {Value::create(10_f), Value::create("ten"s),
                            Value::create(20_f), Value::create("twenty"s)});
        REQUIRE(result->is<Map>());
        const auto& map = result->raw_get<Map>();
        CHECK(map.at(Value::create(10_f))->raw_get<String>() == "ten");
        CHECK(map.at(Value::create(20_f))->raw_get<String>() == "twenty");
    }
}

TEST_CASE("map iteration: Rust reads map via parallel key/value slices")
{
    auto map = Value::create(Value::trusted,
                             Map{{Value::create("x"s), Value::create(1_f)},
                                 {Value::create("y"s), Value::create(2_f)},
                                 {Value::create("z"s), Value::create(3_f)}});

    SECTION("keys")
    {
        auto result = call(frst::rs::test::rt_map_keys_sorted, {map});
        REQUIRE(result->is<Array>());
        CHECK(result->raw_get<Array>().size() == 3);
    }

    SECTION("entry count")
    {
        auto result = call(frst::rs::test::rt_map_entry_count, {map});
        REQUIRE(result->is<Int>());
        CHECK(result->raw_get<Int>() == 3);
    }
}

TEST_CASE("map_get_by_key: Rust looks up by arbitrary key type")
{
    auto map = Value::create(
        Value::trusted,
        Map{{Value::create(42_f), Value::create("found"s)},
            {Value::create("hello"s), Value::create("world"s)}});

    SECTION("int key")
    {
        auto result = call(frst::rs::test::rt_map_get_by_key,
                           {map, Value::create(42_f)});
        REQUIRE(result->is<String>());
        CHECK(result->raw_get<String>() == "found");
    }

    SECTION("string key")
    {
        auto result = call(frst::rs::test::rt_map_get_by_key,
                           {map, Value::create("hello"s)});
        REQUIRE(result->is<String>());
        CHECK(result->raw_get<String>() == "world");
    }

    SECTION("missing key returns null")
    {
        auto result = call(frst::rs::test::rt_map_get_by_key,
                           {map, Value::create(99_f)});
        CHECK(result->is<Null>());
    }
}

// ==========================================================================
// String / binary operations
// ==========================================================================

TEST_CASE("concat_all: variadic string args via slice")
{
    SECTION("two strings")
    {
        auto result = call(frst::rs::test::rt_concat_all,
                           {Value::create("hello"s), Value::create(", world"s)});
        REQUIRE(result->is<String>());
        CHECK(result->raw_get<String>() == "hello, world");
    }

    SECTION("empty args returns empty string")
    {
        auto result = call(frst::rs::test::rt_concat_all, {});
        REQUIRE(result->is<String>());
        CHECK(result->raw_get<String>().empty());
    }
}

TEST_CASE("string_byte_len: Rust accesses raw bytes")
{
    SECTION("ascii")
    {
        auto result = call(frst::rs::test::rt_string_byte_len,
                           {Value::create("hello"s)});
        REQUIRE(result->is<Int>());
        CHECK(result->raw_get<Int>() == 5);
    }

    SECTION("multibyte utf-8")
    {
        auto result = call(frst::rs::test::rt_string_byte_len,
                           {Value::create("\xC3\xA9"s)}); // e-acute, 2 bytes
        REQUIRE(result->is<Int>());
        CHECK(result->raw_get<Int>() == 2);
    }

    SECTION("binary data")
    {
        auto result = call(frst::rs::test::rt_string_byte_len,
                           {Value::create("\x00\x01\x02\xFF"s)});
        REQUIRE(result->is<Int>());
        CHECK(result->raw_get<Int>() == 4);
    }
}

// ==========================================================================
// Unpack (FrostRef enum)
// ==========================================================================

TEST_CASE("describe_type: Rust uses FrostRef enum for pattern matching")
{
    auto check = [](Value_Ptr val, const std::string& expected) {
        auto result = call(frst::rs::test::rt_describe_type, {val});
        REQUIRE(result->is<String>());
        CHECK(result->raw_get<String>() == expected);
    };

    check(Value::null(), "null");
    check(Value::create(42_f), "int:42");
    check(Value::create(Float{3.14}), "float:3.14");
    check(Value::create(Bool{true}), "bool:true");
    check(Value::create("hi"s), "string:2");
    check(Value::create(Array{Value::create(1_f), Value::create(2_f)}),
          "array:2");
    check(Value::create(Value::trusted,
                        Map{{Value::create("k"s), Value::create("v"s)}}),
          "map:1");
}

// ==========================================================================
// Function operations
// ==========================================================================

TEST_CASE("call_callback: Rust calls a Frost function")
{
    auto double_fn = Value::create(
        Function{std::make_shared<Builtin>(
            [](builtin_args_t args) {
                return Value::create(args[0]->raw_get<Int>() * 2);
            },
            "double")});

    auto result = call(frst::rs::test::rt_call_callback,
                       {double_fn, Value::create(21_f)});
    REQUIRE(result->is<Int>());
    CHECK(result->raw_get<Int>() == 42);
}

TEST_CASE("make_adder: Rust creates closures callable from C++")
{
    auto adder = call(frst::rs::test::rt_make_adder, {Value::create(10_f)});
    REQUIRE(adder->is<Function>());

    auto fn = adder->raw_get<Function>();

    SECTION("basic call")
    {
        auto result = fn->call({Value::create(5_f)});
        REQUIRE(result->is<Int>());
        CHECK(result->raw_get<Int>() == 15);
    }

    SECTION("negative")
    {
        auto result = fn->call({Value::create(-3_f)});
        REQUIRE(result->is<Int>());
        CHECK(result->raw_get<Int>() == 7);
    }

    SECTION("zero")
    {
        auto result = fn->call({Value::create(0_f)});
        REQUIRE(result->is<Int>());
        CHECK(result->raw_get<Int>() == 10);
    }
}

TEST_CASE("make_failing_fn: Rust closure Err becomes C++ exception")
{
    auto failing = call(frst::rs::test::rt_make_failing_fn, {});
    REQUIRE(failing->is<Function>());

    auto fn = failing->raw_get<Function>();
    CHECK_THROWS_WITH(fn->call({}),
                      ContainsSubstring("intentional failure"));
}

// ==========================================================================
// Error propagation
// ==========================================================================

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

    SECTION("make_map with odd arg count")
    {
        CHECK_THROWS_WITH(
            call(frst::rs::test::rt_make_map,
                 {Value::create("a"s), Value::create(1_f),
                  Value::create("b"s)}),
            ContainsSubstring("even number"));
    }
}
