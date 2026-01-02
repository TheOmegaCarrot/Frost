#include <catch2/catch_test_macros.hpp>
#include <frost/value.hpp>

using namespace std::literals;
using namespace frst::literals;
using frst::Value, frst::Value_Ptr;

TEST_CASE("Numeric Add")
{
    auto int1 = Value::create(42_f);
    auto int2 = Value::create(81_f);
    auto flt1 = Value::create(3.14);
    auto flt2 = Value::create(2.17);

    SECTION("INT + INT")
    {
        auto res = Value::add(int1, int2);
        REQUIRE(res->is<frst::Int>());
        CHECK(res->get<frst::Int>().value() == 123_f);
    }

    SECTION("FLOAT + FLOAT")
    {
        auto res = Value::add(flt1, flt2);
        REQUIRE(res->is<frst::Float>());
        CHECK(res->get<frst::Float>().value() == 3.14 + 2.17);
    }

    SECTION("INT + FLOAT")
    {
        auto res = Value::add(int1, flt1);
        REQUIRE(res->is<frst::Float>());
        CHECK(res->get<frst::Float>().value() == 42_f + 3.14);
    }

    SECTION("FLOAT + INT")
    {
        auto res = Value::add(flt2, int2);
        REQUIRE(res->is<frst::Float>());
        CHECK(res->get<frst::Float>().value() == 2.17 + 81_f);
    }
}

TEST_CASE("Array Concat")
{
    auto empty = Value::create(frst::Array{});
    auto arr1 = Value::create(frst::Array{
        Value::create(1_f), Value::create(2_f), Value::create(3.14)});
    auto arr2 = Value::create(
        frst::Array{Value::create("Hello"s), Value::create(true)});

    auto arr1_unchanged = [&] {
        CHECK(arr1->get<frst::Array>()->at(0)->get<frst::Int>() == 1_f);
        CHECK(arr1->get<frst::Array>()->at(1)->get<frst::Int>() == 2_f);
        CHECK(arr1->get<frst::Array>()->at(2)->get<frst::Float>() == 3.14);
    };

    SECTION("EMPTY + EMPTY")
    {
        auto res = Value::add(empty, empty);
        REQUIRE(res->is<frst::Array>());
        auto bare_res = res->get<frst::Array>();
        REQUIRE(bare_res->size() == 0);
    }

    SECTION("ARR + EMPTY")
    {
        auto res = Value::add(arr1, empty);
        REQUIRE(res->is<frst::Array>());
        auto bare_res = res->get<frst::Array>();
        REQUIRE(bare_res->size() == 3);
        CHECK(bare_res->at(0)->get<frst::Int>() == 1_f);
        CHECK(bare_res->at(1)->get<frst::Int>() == 2_f);
        CHECK(bare_res->at(2)->get<frst::Float>() == 3.14);

        arr1_unchanged();
    }

    SECTION("EMPTY + ARR")
    {
        auto res = Value::add(empty, arr1);
        REQUIRE(res->is<frst::Array>());
        auto bare_res = res->get<frst::Array>();
        REQUIRE(bare_res->size() == 3);
        CHECK(bare_res->at(0)->get<frst::Int>() == 1_f);
        CHECK(bare_res->at(1)->get<frst::Int>() == 2_f);
        CHECK(bare_res->at(2)->get<frst::Float>() == 3.14);

        arr1_unchanged();
    }

    SECTION("ARR + ARR")
    {
        auto res = Value::add(arr1, arr2);
        REQUIRE(res->is<frst::Array>());
        auto bare_res = res->get<frst::Array>();
        REQUIRE(bare_res->size() == 5);
        CHECK(bare_res->at(0)->get<frst::Int>() == 1_f);
        CHECK(bare_res->at(1)->get<frst::Int>() == 2_f);
        CHECK(bare_res->at(2)->get<frst::Float>() == 3.14);
        CHECK(bare_res->at(3)->get<frst::String>() == "Hello");
        CHECK(bare_res->at(4)->get<frst::Bool>() == true);

        arr1_unchanged();

        CHECK(arr2->get<frst::Array>()->at(0)->get<frst::String>() == "Hello");
        CHECK(arr2->get<frst::Array>()->at(1)->get<frst::Bool>() == true);
    }
}

TEST_CASE("Map Union")
{
    auto empty = Value::create(frst::Map{});
    auto map1 = Value::create(frst::Map{
        {Value::create("foo"s), Value::create(42_f)},
        {Value::create("bar"s), Value::create(81_f)},
        {Value::create("baz"s), Value::create(128_f)},
    });
    auto map2 = Value::create(frst::Map{
        {Value::create("beep"s), Value::create("foo"s)},
        {Value::create("boop"s), Value::create(true)},
        {Value::create("derp"s), Value::create()},
    });
    auto map3 = Value::create(frst::Map{
        {Value::create("foo"s), Value::create(42_f)},
        {Value::create("qux"s), Value::create(128_f)},
    });
}
