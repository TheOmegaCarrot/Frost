#include <catch2/catch_test_macros.hpp>

#include <frost/testing/dummy-callable.hpp>
#include <frost/value.hpp>
#include <memory>

using namespace frst;

using frst::testing::Dummy_Callable;

TEST_CASE("Coercions")
{
    using frst::Value;
    using namespace std::literals;
    using namespace frst::literals;

    Value null{};
    Value integer{42_f};
    Value floating{3.14};
    Value boolean{true};
    Value string{"Hello!"s};
    Value array{frst::Array{Value::create(64.314), Value::create(true)}};
    Value map{frst::Map{
        {
            Value::create("foo"s),
            Value::create(500_f),
        },
        {
            Value::create("bar"s),
            Value::create(100.42),
        },
    }};
    Value function{Function{std::make_shared<Dummy_Callable>()}};

    SECTION("To null")
    {
        CHECK(null.as<frst::Null>().has_value());
        CHECK_FALSE(integer.as<frst::Null>().has_value());
        CHECK_FALSE(floating.as<frst::Null>().has_value());
        CHECK_FALSE(boolean.as<frst::Null>().has_value());
        CHECK_FALSE(string.as<frst::Null>().has_value());
        CHECK_FALSE(array.as<frst::Null>().has_value());
        CHECK_FALSE(map.as<frst::Null>().has_value());
        CHECK_FALSE(function.as<frst::Null>().has_value());
    }

    SECTION("To integer")
    {
        CHECK_FALSE(null.as<frst::Int>().has_value());
        CHECK(integer.as<frst::Int>() == 42);
        CHECK(floating.as<frst::Int>() == 3);
        CHECK_FALSE(boolean.as<frst::Int>().has_value());
        CHECK_FALSE(string.as<frst::Int>().has_value());
        CHECK_FALSE(array.as<frst::Int>().has_value());
        CHECK_FALSE(map.as<frst::Int>().has_value());
        CHECK_FALSE(function.as<frst::Null>().has_value());
    }

    SECTION("To floating")
    {
        CHECK_FALSE(null.as<frst::Float>().has_value());
        CHECK(integer.as<frst::Float>() == static_cast<frst::Float>(42));
        CHECK(floating.as<frst::Float>() == 3.14);
        CHECK_FALSE(boolean.as<frst::Float>().has_value());
        CHECK_FALSE(string.as<frst::Float>().has_value());
        CHECK_FALSE(array.as<frst::Float>().has_value());
        CHECK_FALSE(map.as<frst::Float>().has_value());
        CHECK_FALSE(function.as<frst::Null>().has_value());
    }

    SECTION("To bool")
    {
        CHECK_FALSE(null.as<frst::Bool>().value());
        CHECK(integer.as<frst::Bool>());
        CHECK(floating.as<frst::Bool>());
        CHECK(boolean.as<frst::Bool>());
        CHECK(string.as<frst::Bool>());
        CHECK(array.as<frst::Bool>());
        CHECK(map.as<frst::Bool>());

        Value int_zero{0_f};
        CHECK(int_zero.as<frst::Bool>());

        Value float_zero{0.};
        CHECK(float_zero.as<frst::Bool>());

        Value empty_string{""s};
        CHECK(empty_string.as<frst::Bool>());

        Value bool_false{false};
        CHECK_FALSE(bool_false.as<frst::Bool>().value());

        Value empty_array{frst::Array{}};
        CHECK(empty_array.as<frst::Bool>());

        Value empty_map{frst::Map{}};
        CHECK(empty_map.as<frst::Bool>());
        CHECK(function.as<frst::Bool>());
    }

    SECTION("To string")
    {
        CHECK_FALSE(null.as<frst::String>().has_value());
        CHECK_FALSE(integer.as<frst::String>().has_value());
        CHECK_FALSE(floating.as<frst::String>().has_value());
        CHECK_FALSE(boolean.as<frst::String>().has_value());
        CHECK(string.as<frst::String>() == "Hello!");
        CHECK_FALSE(array.as<frst::String>().has_value());
        CHECK_FALSE(map.as<frst::String>().has_value());
        CHECK_FALSE(function.as<frst::String>().has_value());
    }

    SECTION("To array")
    {
        CHECK_FALSE(null.as<frst::Array>().has_value());
        CHECK_FALSE(integer.as<frst::Array>().has_value());
        CHECK_FALSE(floating.as<frst::Array>().has_value());
        CHECK_FALSE(boolean.as<frst::Array>().has_value());
        CHECK_FALSE(string.as<frst::Array>().has_value());
        CHECK(array.as<frst::Array>() == array.get<frst::Array>());
        CHECK_FALSE(map.as<frst::Array>().has_value());
        CHECK_FALSE(function.as<frst::Array>().has_value());
    }

    SECTION("To map")
    {
        CHECK_FALSE(null.as<frst::Map>().has_value());
        CHECK_FALSE(integer.as<frst::Map>().has_value());
        CHECK_FALSE(floating.as<frst::Map>().has_value());
        CHECK_FALSE(boolean.as<frst::Map>().has_value());
        CHECK_FALSE(string.as<frst::Map>().has_value());
        CHECK_FALSE(array.as<frst::Map>().has_value());
        CHECK(map.as<frst::Map>() == map.get<frst::Map>());
        CHECK_FALSE(function.as<frst::Map>().has_value());
    }

    SECTION("To function")
    {
        CHECK_FALSE(null.as<frst::Function>().has_value());
        CHECK_FALSE(integer.as<frst::Function>().has_value());
        CHECK_FALSE(floating.as<frst::Function>().has_value());
        CHECK_FALSE(boolean.as<frst::Function>().has_value());
        CHECK_FALSE(string.as<frst::Function>().has_value());
        CHECK_FALSE(array.as<frst::Function>().has_value());
        CHECK_FALSE(map.as<frst::Function>().has_value());
        CHECK(function.as<frst::Function>() == function.get<frst::Function>());
    }
}
