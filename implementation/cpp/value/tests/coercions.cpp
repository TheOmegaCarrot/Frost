#include <catch2/catch_test_macros.hpp>
#include <frost/value.hpp>
#include <memory>

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

    SECTION("To null")
    {
        CHECK(null.as<frst::Null>().has_value());
        CHECK(!integer.as<frst::Null>().has_value());
        CHECK(!floating.as<frst::Null>().has_value());
        CHECK(!boolean.as<frst::Null>().has_value());
        CHECK(!string.as<frst::Null>().has_value());
        CHECK(!array.as<frst::Null>().has_value());
        CHECK(!map.as<frst::Null>().has_value());
    }

    SECTION("To integer")
    {
        CHECK(!null.as<frst::Int>().has_value());
        CHECK(integer.as<frst::Int>() == 42);
        CHECK(floating.as<frst::Int>() == 3);
        CHECK(!boolean.as<frst::Int>().has_value());
        CHECK(!string.as<frst::Int>().has_value());
        CHECK(!array.as<frst::Int>().has_value());
        CHECK(!map.as<frst::Int>().has_value());
    }

    SECTION("To floating")
    {
        CHECK(!null.as<frst::Float>().has_value());
        CHECK(integer.as<frst::Float>() == static_cast<frst::Float>(42));
        CHECK(floating.as<frst::Float>() == 3.14);
        CHECK(!boolean.as<frst::Float>().has_value());
        CHECK(!string.as<frst::Float>().has_value());
        CHECK(!array.as<frst::Float>().has_value());
        CHECK(!map.as<frst::Float>().has_value());
    }

    SECTION("To bool")
    {
        CHECK(null.as<frst::Bool>() == false);
        CHECK(integer.as<frst::Bool>() == true);
        CHECK(floating.as<frst::Bool>() == true);
        CHECK(boolean.as<frst::Bool>() == true);
        CHECK(string.as<frst::Bool>() == true);
        CHECK(array.as<frst::Bool>() == true);
        CHECK(map.as<frst::Bool>() == true);

        Value int_zero{0_f};
        CHECK(int_zero.as<frst::Bool>() == true);

        Value float_zero{0.};
        CHECK(float_zero.as<frst::Bool>() == true);

        Value empty_string{""s};
        CHECK(empty_string.as<frst::Bool>() == true);

        Value bool_false{false};
        CHECK(bool_false.as<frst::Bool>() == false);

        Value empty_array{frst::Array{}};
        CHECK(empty_array.as<frst::Bool>() == true);

        Value empty_map{frst::Map{}};
        CHECK(empty_map.as<frst::Bool>() == true);
    }

    SECTION("To string")
    {
        CHECK(!null.as<frst::String>().has_value());
        CHECK(!integer.as<frst::String>().has_value());
        CHECK(!floating.as<frst::String>().has_value());
        CHECK(!boolean.as<frst::String>().has_value());
        CHECK(string.as<frst::String>() == "Hello!");
        CHECK(!array.as<frst::String>().has_value());
        CHECK(!map.as<frst::String>().has_value());
    }

    SECTION("To array")
    {
        CHECK(!null.as<frst::Array>().has_value());
        CHECK(!integer.as<frst::Array>().has_value());
        CHECK(!floating.as<frst::Array>().has_value());
        CHECK(!boolean.as<frst::Array>().has_value());
        CHECK(!string.as<frst::Array>().has_value());
        CHECK(array.as<frst::Array>() == array.as<frst::Array>());
        CHECK(!map.as<frst::Array>().has_value());
    }

    SECTION("To map")
    {
        CHECK(!null.as<frst::Map>().has_value());
        CHECK(!integer.as<frst::Map>().has_value());
        CHECK(!floating.as<frst::Map>().has_value());
        CHECK(!boolean.as<frst::Map>().has_value());
        CHECK(!string.as<frst::Map>().has_value());
        CHECK(!array.as<frst::Map>().has_value());
        CHECK(map.as<frst::Map>() == map.as<frst::Map>());
    }
}
