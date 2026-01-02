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

    auto all_unchanged = [&] {
        CHECK(empty->get<frst::Array>()->size() == 0);

        CHECK(arr1->get<frst::Array>()->at(0)->get<frst::Int>() == 1_f);
        CHECK(arr1->get<frst::Array>()->at(1)->get<frst::Int>() == 2_f);
        CHECK(arr1->get<frst::Array>()->at(2)->get<frst::Float>() == 3.14);

        CHECK(arr2->get<frst::Array>()->at(0)->get<frst::String>() == "Hello");
        CHECK(arr2->get<frst::Array>()->at(1)->get<frst::Bool>() == true);
    };

    SECTION("EMPTY + EMPTY")
    {
        auto res = Value::add(empty, empty);
        REQUIRE(res->is<frst::Array>());
        auto bare_res = res->get<frst::Array>();
        CHECK(bare_res->size() == 0);

        all_unchanged();
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

        all_unchanged();
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

        all_unchanged();
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

        all_unchanged();
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
        {Value::create("foo"s), Value::create(510_f)},
        {Value::create("qux"s), Value::create(2038_f)},
    });

    auto all_unchanged = [&] {
        CHECK(empty->get<frst::Map>()->size() == 0);

        CHECK(map1->get<frst::Map>()->size() == 3);
        for (const auto& [k, v] : *map1->get<frst::Map>())
        {
            if (k->as<frst::String>() == "foo")
                CHECK(v->as<frst::Int>() == 42_f);
            if (k->as<frst::String>() == "bar")
                CHECK(v->as<frst::Int>() == 81_f);
            if (k->as<frst::String>() == "baz")
                CHECK(v->as<frst::Int>() == 128_f);
        }

        CHECK(map2->get<frst::Map>()->size() == 3);
        for (const auto& [k, v] : *map2->get<frst::Map>())
        {
            if (k->as<frst::String>() == "beep")
                CHECK(v->as<frst::String>() == "foo");
            if (k->as<frst::String>() == "boop")
                CHECK(v->as<frst::Bool>() == true);
            if (k->as<frst::String>() == "derp")
                CHECK(v->is<frst::Null>());
        }

        CHECK(map3->get<frst::Map>()->size() == 2);
        for (const auto& [k, v] : *map3->get<frst::Map>())
        {
            if (k->as<frst::String>() == "foo")
                CHECK(v->as<frst::Int>() == 510_f);
            if (k->as<frst::String>() == "qux")
                CHECK(v->as<frst::Int>() == 2038_f);
        }
    };

    SECTION("EMPTY + MAP")
    {
        auto res = Value::add(empty, empty);

        CHECK(res->get<frst::Map>()->size() == 0);

        all_unchanged();
    }

    SECTION("EMPTY + MAP")
    {
        auto res = Value::add(empty, map1);

        CHECK(res->get<frst::Map>()->size() == 3);
        for (const auto& [k, v] : *res->get<frst::Map>())
        {
            if (k->as<frst::String>() == "foo")
                CHECK(v->as<frst::Int>() == 42_f);
            if (k->as<frst::String>() == "bar")
                CHECK(v->as<frst::Int>() == 81_f);
            if (k->as<frst::String>() == "baz")
                CHECK(v->as<frst::Int>() == 128_f);
        }

        all_unchanged();
    }

    SECTION("MAP + EMPTY")
    {
        auto res = Value::add(map1, empty);

        CHECK(res->get<frst::Map>()->size() == 3);
        for (const auto& [k, v] : *res->get<frst::Map>())
        {
            if (k->as<frst::String>() == "foo")
                CHECK(v->as<frst::Int>() == 42_f);
            if (k->as<frst::String>() == "bar")
                CHECK(v->as<frst::Int>() == 81_f);
            if (k->as<frst::String>() == "baz")
                CHECK(v->as<frst::Int>() == 128_f);
        }

        all_unchanged();
    }

    SECTION("MAP + MAP (No collision)")
    {
        auto res = Value::add(map1, map2);

        CHECK(res->get<frst::Map>()->size() == 6);
        for (const auto& [k, v] : *res->get<frst::Map>())
        {
            if (k->as<frst::String>() == "foo")
                CHECK(v->as<frst::Int>() == 42_f);
            if (k->as<frst::String>() == "bar")
                CHECK(v->as<frst::Int>() == 81_f);
            if (k->as<frst::String>() == "baz")
                CHECK(v->as<frst::Int>() == 128_f);
            if (k->as<frst::String>() == "beep")
                CHECK(v->as<frst::String>() == "foo");
            if (k->as<frst::String>() == "boop")
                CHECK(v->as<frst::Bool>() == true);
            if (k->as<frst::String>() == "derp")
                CHECK(v->is<frst::Null>());
        }

        all_unchanged();
    }

    SECTION("MAP + MAP (With collision)")
    {
        auto res = Value::add(map1, map3);

        CHECK(res->get<frst::Map>()->size() == 4);
        for (const auto& [k, v] : *res->get<frst::Map>())
        {
            if (k->as<frst::String>() == "foo")
                CHECK(v->as<frst::Int>() == 510_f);
            if (k->as<frst::String>() == "bar")
                CHECK(v->as<frst::Int>() == 81_f);
            if (k->as<frst::String>() == "baz")
                CHECK(v->as<frst::Int>() == 128_f);
            if (k->as<frst::String>() == "qux")
                CHECK(v->as<frst::Int>() == 2038_f);
        }

        all_unchanged();
    }
}
