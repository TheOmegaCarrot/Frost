#include <catch2/catch_test_macros.hpp>
#include <frost/value.hpp>
#include <frost/testing/dummy-callable.hpp>

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
        Value value{};
        CHECK(value.is<frst::Null>());
        CHECK_FALSE(value.is_numeric());
        CHECK(value.is_primitive());
        CHECK_FALSE(value.is_structured());
    }

    SECTION("Null")
    {
        Value value{frst::Null{}};
        CHECK(value.is<frst::Null>());
        CHECK_FALSE(value.is_numeric());
        CHECK(value.is_primitive());
        CHECK_FALSE(value.is_structured());
    }

    SECTION("Int")
    {
        using namespace frst::literals;
        Value value{42_f};
        CHECK(value.is<frst::Int>());
        CHECK(value.is_numeric());
        CHECK(value.is_primitive());
        CHECK_FALSE(value.is_structured());
    }

    SECTION("Float")
    {
        Value value{frst::Float{3.14}};
        CHECK(value.is<frst::Float>());
        CHECK(value.is_numeric());
        CHECK(value.is_primitive());
        CHECK_FALSE(value.is_structured());
    }

    SECTION("Bool")
    {
        Value value{true};
        CHECK(value.is<frst::Bool>());
        CHECK_FALSE(value.is_numeric());
        CHECK(value.is_primitive());
        CHECK_FALSE(value.is_structured());
    }

    SECTION("String")
    {
        Value value{"Hello"s};
        CHECK(value.is<frst::String>());
        CHECK_FALSE(value.is_numeric());
        CHECK(value.is_primitive());
        CHECK_FALSE(value.is_structured());
    }

    SECTION("Array")
    {
        Value value{frst::Array{}};
        CHECK(value.is<frst::Array>());
        CHECK_FALSE(value.is_numeric());
        CHECK_FALSE(value.is_primitive());
        CHECK(value.is_structured());
    }

    SECTION("Map")
    {
        Value value{frst::Map{}};
        CHECK(value.is<frst::Map>());
        CHECK_FALSE(value.is_numeric());
        CHECK_FALSE(value.is_primitive());
        CHECK(value.is_structured());
    }

    SECTION("Function")
    {
        Value value{frst::Function{std::make_shared<Dummy_Callable>()}};
        CHECK(value.is<frst::Function>());
        CHECK_FALSE(value.is_numeric());
        CHECK_FALSE(value.is_primitive());
        CHECK_FALSE(value.is_structured());
    }
}

TEST_CASE("Get")
{
    using namespace frst::literals;
    Value null{};
    Value integer{42_f};
    Value floating{3.14};
    Value boolean{true};
    Value string{"Hello"s};
    Value array{frst::Array{Value::create(42_f)}};
    Value map{frst::Map{{Value::create(42_f), Value::create("Hello"s)}}};
    Value function{frst::Function{std::make_shared<Dummy_Callable>()}};

    SECTION("Null")
    {
        CHECK(null.get<frst::Null>().has_value());
        CHECK(!null.get<frst::Int>().has_value());
        CHECK(!null.get<frst::Float>().has_value());
        CHECK(!null.get<frst::String>().has_value());
        CHECK(!null.get<frst::Bool>().has_value());
        CHECK(!null.get<frst::Array>().has_value());
        CHECK(!null.get<frst::Map>().has_value());
    }

    SECTION("Int")
    {
        CHECK(!integer.get<frst::Null>().has_value());
        CHECK(integer.get<frst::Int>() == 42_f);
        CHECK(!integer.get<frst::Float>().has_value());
        CHECK(!integer.get<frst::String>().has_value());
        CHECK(!integer.get<frst::Bool>().has_value());
        CHECK(!integer.get<frst::Array>().has_value());
        CHECK(!integer.get<frst::Map>().has_value());
    }

    SECTION("Float")
    {
        CHECK(!floating.get<frst::Null>().has_value());
        CHECK(!floating.get<frst::Int>().has_value());
        CHECK(floating.get<frst::Float>() == 3.14);
        CHECK(!floating.get<frst::String>().has_value());
        CHECK(!floating.get<frst::Bool>().has_value());
        CHECK(!floating.get<frst::Array>().has_value());
        CHECK(!floating.get<frst::Map>().has_value());
    }

    SECTION("String")
    {
        CHECK(!string.get<frst::Null>().has_value());
        CHECK(!string.get<frst::Int>().has_value());
        CHECK(!string.get<frst::Float>().has_value());
        CHECK(string.get<frst::String>() == "Hello");
        CHECK(!string.get<frst::Bool>().has_value());
        CHECK(!string.get<frst::Array>().has_value());
        CHECK(!string.get<frst::Map>().has_value());
    }

    SECTION("Bool")
    {
        CHECK(!boolean.get<frst::Null>().has_value());
        CHECK(!boolean.get<frst::Int>().has_value());
        CHECK(!boolean.get<frst::Float>().has_value());
        CHECK(!boolean.get<frst::String>().has_value());
        CHECK(boolean.get<frst::Bool>() == true);
        CHECK(!boolean.get<frst::Array>().has_value());
        CHECK(!boolean.get<frst::Map>().has_value());
    }

    SECTION("Array")
    {
        CHECK(!array.get<frst::Null>().has_value());
        CHECK(!array.get<frst::Int>().has_value());
        CHECK(!array.get<frst::Float>().has_value());
        CHECK(!array.get<frst::String>().has_value());
        CHECK(!array.get<frst::Bool>().has_value());
        CHECK(array.get<frst::Array>().has_value());
        CHECK(!array.get<frst::Map>().has_value());
    }

    SECTION("Map")
    {
        CHECK(!map.get<frst::Null>().has_value());
        CHECK(!map.get<frst::Int>().has_value());
        CHECK(!map.get<frst::Float>().has_value());
        CHECK(!map.get<frst::String>().has_value());
        CHECK(!map.get<frst::Bool>().has_value());
        CHECK(!map.get<frst::Array>().has_value());
        CHECK(map.get<frst::Map>().has_value());
    }

    SECTION("Function")
    {
        CHECK(!function.get<frst::Null>().has_value());
        CHECK(!function.get<frst::Int>().has_value());
        CHECK(!function.get<frst::Float>().has_value());
        CHECK(!function.get<frst::String>().has_value());
        CHECK(!function.get<frst::Bool>().has_value());
        CHECK(!function.get<frst::Array>().has_value());
        CHECK(!function.get<frst::Map>().has_value());
        CHECK(function.get<frst::Function>().has_value());
    }
}

TEST_CASE("Type Name")
{
    using namespace frst::literals;
    Value null{};
    Value integer{42_f};
    Value floating{3.14};
    Value boolean{true};
    Value string{"Hello"s};
    Value array{frst::Array{Value::create(42_f)}};
    Value map{frst::Map{{Value::create(42_f), Value::create("Hello"s)}}};
    Value function{frst::Function{std::make_shared<Dummy_Callable>()}};

    CHECK(null.type_name() == "Null");
    CHECK(integer.type_name() == "Int");
    CHECK(floating.type_name() == "Float");
    CHECK(boolean.type_name() == "Bool");
    CHECK(string.type_name() == "String");
    CHECK(array.type_name() == "Array");
    CHECK(map.type_name() == "Map");
    CHECK(function.type_name() == "Function");
}
