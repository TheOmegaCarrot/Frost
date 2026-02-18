#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>
#include <frost/testing/dummy-callable.hpp>
#include <frost/value.hpp>

#include <cmath>
#include <memory>

using namespace std::literals;
using frst::Value;

namespace
{
// AI-generated test additions by Codex (GPT-5).
// Signed: Codex (GPT-5).
using frst::testing::Dummy_Callable;
} // namespace

TEST_CASE("Construction and .is*<T>")
{
    SECTION("Default")
    {
        const auto value = Value::null();
        CHECK(value->is<frst::Null>());
        CHECK_FALSE(value->is_numeric());
        CHECK(value->is_primitive());
        CHECK_FALSE(value->is_structured());
    }

    SECTION("Null")
    {
        const auto value = Value::create(frst::Null{});
        CHECK(value->is<frst::Null>());
        CHECK_FALSE(value->is_numeric());
        CHECK(value->is_primitive());
        CHECK_FALSE(value->is_structured());
    }

    SECTION("Int")
    {
        using namespace frst::literals;
        const auto value = Value::create(42_f);
        CHECK(value->is<frst::Int>());
        CHECK(value->is_numeric());
        CHECK(value->is_primitive());
        CHECK_FALSE(value->is_structured());
    }

    SECTION("Float")
    {
        const auto value = Value::create(frst::Float{3.14});
        CHECK(value->is<frst::Float>());
        CHECK(value->is_numeric());
        CHECK(value->is_primitive());
        CHECK_FALSE(value->is_structured());
    }

    SECTION("Float rejects NaN")
    {
        const auto nan = std::numeric_limits<frst::Float>::quiet_NaN();
        CHECK_THROWS(Value::create(frst::Float{nan}));
    }

    SECTION("Float rejects Infinity")
    {
        const auto inf = std::numeric_limits<frst::Float>::infinity();
        CHECK_THROWS(Value::create(frst::Float{inf}));
        CHECK_THROWS(Value::create(frst::Float{-inf}));
    }

    SECTION("Bool")
    {
        const auto value = Value::create(true);
        CHECK(value->is<frst::Bool>());
        CHECK_FALSE(value->is_numeric());
        CHECK(value->is_primitive());
        CHECK_FALSE(value->is_structured());
    }

    SECTION("String")
    {
        const auto value = Value::create("Hello"s);
        CHECK(value->is<frst::String>());
        CHECK_FALSE(value->is_numeric());
        CHECK(value->is_primitive());
        CHECK_FALSE(value->is_structured());
    }

    SECTION("Array")
    {
        const auto value = Value::create(frst::Array{});
        CHECK(value->is<frst::Array>());
        CHECK_FALSE(value->is_numeric());
        CHECK_FALSE(value->is_primitive());
        CHECK(value->is_structured());
    }

    SECTION("Map")
    {
        const auto value = Value::create(frst::Map{});
        CHECK(value->is<frst::Map>());
        CHECK_FALSE(value->is_numeric());
        CHECK_FALSE(value->is_primitive());
        CHECK(value->is_structured());
    }

    SECTION("Map key restrictions")
    {
        CHECK_NOTHROW(Value::create(
            frst::Map{{Value::create(frst::Int{42}), Value::create("int"s)}}));
        CHECK_NOTHROW(Value::create(
            frst::Map{{Value::create(3.5), Value::create("float"s)}}));
        CHECK_NOTHROW(Value::create(
            frst::Map{{Value::create(true), Value::create("bool"s)}}));
        CHECK_NOTHROW(Value::create(
            frst::Map{{Value::create("k"s), Value::create("string"s)}}));

        auto fn_key = Value::create(
            frst::Function{std::make_shared<Dummy_Callable>()});

        CHECK_THROWS_WITH(
            Value::create(
                frst::Map{{Value::null(), Value::create(frst::Int{1})}}),
            Catch::Matchers::ContainsSubstring(
                "Map keys may only be primitive values"));
        CHECK_THROWS_WITH(
            Value::create(frst::Map{{Value::create(frst::Array{}),
                                     Value::create(frst::Int{1})}}),
            Catch::Matchers::ContainsSubstring(
                "Map keys may only be primitive values"));
        CHECK_THROWS_WITH(
            Value::create(frst::Map{
                {Value::create(frst::Map{}), Value::create(frst::Int{1})}}),
            Catch::Matchers::ContainsSubstring(
                "Map keys may only be primitive values"));
        CHECK_THROWS_WITH(
            Value::create(frst::Map{{fn_key, Value::create(frst::Int{1})}}),
            Catch::Matchers::ContainsSubstring(
                "Map keys may only be primitive values"));
    }

    SECTION("Function")
    {
        const auto value =
            Value::create(frst::Function{std::make_shared<Dummy_Callable>()});
        CHECK(value->is<frst::Function>());
        CHECK_FALSE(value->is_numeric());
        CHECK_FALSE(value->is_primitive());
        CHECK_FALSE(value->is_structured());
    }
}

TEST_CASE("Get")
{
    using namespace frst::literals;
    const auto null = Value::null();
    const auto integer = Value::create(42_f);
    const auto floating = Value::create(3.14);
    const auto boolean = Value::create(true);
    const auto string = Value::create("Hello"s);
    const auto array = Value::create(frst::Array{Value::create(42_f)});
    const auto map =
        Value::create(frst::Map{{Value::create(42_f), Value::create("Hello"s)}});
    const auto function =
        Value::create(frst::Function{std::make_shared<Dummy_Callable>()});

    SECTION("Null")
    {
        CHECK(null->get<frst::Null>().has_value());
        CHECK(!null->get<frst::Int>().has_value());
        CHECK(!null->get<frst::Float>().has_value());
        CHECK(!null->get<frst::String>().has_value());
        CHECK(!null->get<frst::Bool>().has_value());
        CHECK(!null->get<frst::Array>().has_value());
        CHECK(!null->get<frst::Map>().has_value());
    }

    SECTION("Int")
    {
        CHECK(!integer->get<frst::Null>().has_value());
        CHECK(integer->get<frst::Int>() == 42_f);
        CHECK(!integer->get<frst::Float>().has_value());
        CHECK(!integer->get<frst::String>().has_value());
        CHECK(!integer->get<frst::Bool>().has_value());
        CHECK(!integer->get<frst::Array>().has_value());
        CHECK(!integer->get<frst::Map>().has_value());
    }

    SECTION("Float")
    {
        CHECK(!floating->get<frst::Null>().has_value());
        CHECK(!floating->get<frst::Int>().has_value());
        CHECK(floating->get<frst::Float>() == 3.14);
        CHECK(!floating->get<frst::String>().has_value());
        CHECK(!floating->get<frst::Bool>().has_value());
        CHECK(!floating->get<frst::Array>().has_value());
        CHECK(!floating->get<frst::Map>().has_value());
    }

    SECTION("String")
    {
        CHECK(!string->get<frst::Null>().has_value());
        CHECK(!string->get<frst::Int>().has_value());
        CHECK(!string->get<frst::Float>().has_value());
        CHECK(string->get<frst::String>() == "Hello");
        CHECK(!string->get<frst::Bool>().has_value());
        CHECK(!string->get<frst::Array>().has_value());
        CHECK(!string->get<frst::Map>().has_value());
    }

    SECTION("Bool")
    {
        CHECK(!boolean->get<frst::Null>().has_value());
        CHECK(!boolean->get<frst::Int>().has_value());
        CHECK(!boolean->get<frst::Float>().has_value());
        CHECK(!boolean->get<frst::String>().has_value());
        CHECK(boolean->get<frst::Bool>() == true);
        CHECK(!boolean->get<frst::Array>().has_value());
        CHECK(!boolean->get<frst::Map>().has_value());
    }

    SECTION("Array")
    {
        CHECK(!array->get<frst::Null>().has_value());
        CHECK(!array->get<frst::Int>().has_value());
        CHECK(!array->get<frst::Float>().has_value());
        CHECK(!array->get<frst::String>().has_value());
        CHECK(!array->get<frst::Bool>().has_value());
        CHECK(array->get<frst::Array>().has_value());
        CHECK(!array->get<frst::Map>().has_value());
    }

    SECTION("Map")
    {
        CHECK(!map->get<frst::Null>().has_value());
        CHECK(!map->get<frst::Int>().has_value());
        CHECK(!map->get<frst::Float>().has_value());
        CHECK(!map->get<frst::String>().has_value());
        CHECK(!map->get<frst::Bool>().has_value());
        CHECK(!map->get<frst::Array>().has_value());
        CHECK(map->get<frst::Map>().has_value());
    }

    SECTION("Function")
    {
        CHECK(!function->get<frst::Null>().has_value());
        CHECK(!function->get<frst::Int>().has_value());
        CHECK(!function->get<frst::Float>().has_value());
        CHECK(!function->get<frst::String>().has_value());
        CHECK(!function->get<frst::Bool>().has_value());
        CHECK(!function->get<frst::Array>().has_value());
        CHECK(!function->get<frst::Map>().has_value());
        CHECK(function->get<frst::Function>().has_value());
    }
}

TEST_CASE("Type Name")
{
    using namespace frst::literals;
    const auto null = Value::null();
    const auto integer = Value::create(42_f);
    const auto floating = Value::create(3.14);
    const auto boolean = Value::create(true);
    const auto string = Value::create("Hello"s);
    const auto array = Value::create(frst::Array{Value::create(42_f)});
    const auto map =
        Value::create(frst::Map{{Value::create(42_f), Value::create("Hello"s)}});
    const auto function =
        Value::create(frst::Function{std::make_shared<Dummy_Callable>()});

    CHECK(null->type_name() == "Null");
    CHECK(integer->type_name() == "Int");
    CHECK(floating->type_name() == "Float");
    CHECK(boolean->type_name() == "Bool");
    CHECK(string->type_name() == "String");
    CHECK(array->type_name() == "Array");
    CHECK(map->type_name() == "Map");
    CHECK(function->type_name() == "Function");
}

TEST_CASE("Singleton identity")
{
    SECTION("Null singleton")
    {
        const auto null_a = Value::null();
        const auto null_b = Value::null();
        const auto null_c = Value::create(frst::Null{});

        CHECK(null_a == null_b);
        CHECK(null_c == null_a);
        CHECK(null_a->get<frst::Null>().has_value());
    }

    SECTION("Bool singletons")
    {
        const auto true_a = Value::create(true);
        const auto true_b = Value::create(true);
        const auto false_a = Value::create(false);
        const auto false_b = Value::create(false);

        CHECK(true_a == true_b);
        CHECK(false_a == false_b);
        CHECK(true_a != false_a);

        CHECK(true_a->get<frst::Bool>().has_value());
        CHECK(false_a->get<frst::Bool>().has_value());
        CHECK(true_a->get<frst::Bool>().value());
        CHECK_FALSE(false_a->get<frst::Bool>().value());
    }
}
