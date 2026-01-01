#include <catch2/catch_test_macros.hpp>
#include <frost/value.hpp>

using namespace std::literals;
using frst::Value;

TEST_CASE("Construction and .is<T>")
{
    SECTION("Default")
    {
        Value value{};
        CHECK(value.is<frst::Null>());
    }

    SECTION("Null")
    {
        Value value{frst::Null{}};
        CHECK(value.is<frst::Null>());
    }

    SECTION("Int")
    {
        using namespace frst::literals;
        Value value{42_f};
        CHECK(value.is<frst::Int>());
    }

    SECTION("Float")
    {
        Value value{frst::Float{3.14}};
        CHECK(value.is<frst::Float>());
    }

    SECTION("Bool")
    {
        Value value{true};
        CHECK(value.is<frst::Bool>());
    }

    SECTION("String")
    {
        Value value{"Hello"s};
        CHECK(value.is<frst::String>());
    }

    SECTION("Array")
    {
        Value value{frst::Array{}};
        CHECK(value.is<frst::Array>());
    }

    SECTION("Map")
    {
        Value value{frst::Map{}};
        CHECK(value.is<frst::Map>());
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
    Value array{frst::Array{std::make_shared<Value>(42_f)}};
    Value map{frst::Map{
        {std::make_shared<Value>(42_f), std::make_shared<Value>("Hello"s)}}};

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
}
