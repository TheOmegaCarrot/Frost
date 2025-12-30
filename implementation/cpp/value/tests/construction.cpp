#include <catch2/catch_test_macros.hpp>
#include <frost/value.hpp>

TEST_CASE("Construction")
{
    SECTION("Default")
    {
        frst::Value value{};
        CHECK(value.is<frst::Null>());
    }

    SECTION("Null")
    {
        frst::Value value{frst::Null{}};
        CHECK(value.is<frst::Null>());
    }

    SECTION("Int")
    {
        frst::Value value{frst::Int{42}};
        CHECK(value.is<frst::Int>());
    }

    SECTION("Float")
    {
        frst::Value value{frst::Float{3.14}};
        CHECK(value.is<frst::Float>());
    }

    SECTION("Bool")
    {
        frst::Value value{true};
        CHECK(value.is<frst::Bool>());
    }

    using namespace std::literals;

    SECTION("String")
    {
        frst::Value value{"Hello"s};
        CHECK(value.is<frst::String>());
    }

    SECTION("Array")
    {
        frst::Value value{frst::Array{}};
        CHECK(value.is<frst::Array>());
    }

    SECTION("Map")
    {
        frst::Value value{frst::Map{}};
        CHECK(value.is<frst::Map>());
    }
}
