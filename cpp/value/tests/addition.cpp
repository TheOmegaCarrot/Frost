#include <catch2/catch_all.hpp>

#include "op-test-macros.hpp"

#include <memory>

#include <frost/testing/dummy-callable.hpp>
#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/value.hpp>

using namespace std::literals;
using namespace frst::literals;
using frst::Value, frst::Value_Ptr;

namespace
{
// AI-generated test additions by Codex (GPT-5).
// Signed: Codex (GPT-5).
using frst::testing::Dummy_Callable;
} // namespace

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

TEST_CASE("String Concat")
{
    auto empty = Value::create(""s);
    auto foo = Value::create("foo"s);
    auto bar = Value::create("bar"s);

    auto all_unchanged = [&] {
        CHECK(empty->get<frst::String>() == ""s);
        CHECK(foo->get<frst::String>() == "foo"s);
        CHECK(bar->get<frst::String>() == "bar"s);
    };

    SECTION("EMPTY + EMPTY")
    {
        CHECK(Value::add(empty, empty)->get<frst::String>() == ""s);
        all_unchanged();
    }

    SECTION("STR + EMPTY")
    {
        CHECK(Value::add(foo, empty)->get<frst::String>() == "foo"s);
        all_unchanged();
    }

    SECTION("EMPTY + STR")
    {
        CHECK(Value::add(empty, foo)->get<frst::String>() == "foo"s);
        all_unchanged();
    }

    SECTION("STR + STR")
    {
        CHECK(Value::add(foo, bar)->get<frst::String>() == "foobar"s);
        all_unchanged();
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
        {Value::create("derp"s), Value::null()},
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

    SECTION("EMPTY + EMPTY")
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

        // Unioning maps with key collisions should result in
        // zero duplicate keys
        // And the union should take the value from the second map
        //
        // This should be based on the underlying value,
        // rather than the identity of the value
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

    SECTION("MAP + MAP (Interleaved collisions and tails)")
    {
        auto lhs = Value::create(frst::Map{
            {Value::create("a"s), Value::create(1_f)},
            {Value::create("c"s), Value::create(3_f)},
            {Value::create("e"s), Value::create(5_f)},
        });
        auto rhs = Value::create(frst::Map{
            {Value::create("a"s), Value::create(10_f)},
            {Value::create("d"s), Value::create(40_f)},
            {Value::create("e"s), Value::create(50_f)},
            {Value::create("z"s), Value::create(260_f)},
        });

        auto res = Value::add(lhs, rhs);
        const auto merged_opt = res->get<frst::Map>();
        REQUIRE(merged_opt.has_value());
        const auto& merged = merged_opt.value();

        CHECK(merged.size() == 5);

        const auto a = merged.find(Value::create("a"s));
        const auto c = merged.find(Value::create("c"s));
        const auto d = merged.find(Value::create("d"s));
        const auto e = merged.find(Value::create("e"s));
        const auto z = merged.find(Value::create("z"s));

        REQUIRE(a != merged.end());
        REQUIRE(c != merged.end());
        REQUIRE(d != merged.end());
        REQUIRE(e != merged.end());
        REQUIRE(z != merged.end());

        CHECK(a->second->as<frst::Int>() == 10_f);
        CHECK(c->second->as<frst::Int>() == 3_f);
        CHECK(d->second->as<frst::Int>() == 40_f);
        CHECK(e->second->as<frst::Int>() == 50_f);
        CHECK(z->second->as<frst::Int>() == 260_f);
    }

    SECTION("MAP + MAP (All keys collide)")
    {
        auto lhs = Value::create(frst::Map{
            {Value::create("x"s), Value::create(1_f)},
            {Value::create("y"s), Value::create(2_f)},
            {Value::create("z"s), Value::create(3_f)},
        });
        auto rhs = Value::create(frst::Map{
            {Value::create("x"s), Value::create(100_f)},
            {Value::create("y"s), Value::create(200_f)},
            {Value::create("z"s), Value::create(300_f)},
        });

        auto res = Value::add(lhs, rhs);
        const auto merged_opt = res->get<frst::Map>();
        REQUIRE(merged_opt.has_value());
        const auto& merged = merged_opt.value();

        CHECK(merged.size() == 3);

        const auto x = merged.find(Value::create("x"s));
        const auto y = merged.find(Value::create("y"s));
        const auto z = merged.find(Value::create("z"s));

        REQUIRE(x != merged.end());
        REQUIRE(y != merged.end());
        REQUIRE(z != merged.end());

        CHECK(x->second->as<frst::Int>() == 100_f);
        CHECK(y->second->as<frst::Int>() == 200_f);
        CHECK(z->second->as<frst::Int>() == 300_f);
    }

    SECTION("MAP + MAP (LHS shorter, RHS tail append)")
    {
        auto lhs = Value::create(frst::Map{
            {Value::create("a"s), Value::create(1_f)},
            {Value::create("b"s), Value::create(2_f)},
        });
        auto rhs = Value::create(frst::Map{
            {Value::create("c"s), Value::create(3_f)},
            {Value::create("d"s), Value::create(4_f)},
            {Value::create("e"s), Value::create(5_f)},
            {Value::create("f"s), Value::create(6_f)},
            {Value::create("g"s), Value::create(7_f)},
            {Value::create("h"s), Value::create(8_f)},
            {Value::create("i"s), Value::create(9_f)},
        });

        auto res = Value::add(lhs, rhs);
        const auto merged_opt = res->get<frst::Map>();
        REQUIRE(merged_opt.has_value());
        const auto& merged = merged_opt.value();

        CHECK(merged.size() == 9);
        CHECK(merged.at(Value::create("a"s))->as<frst::Int>() == 1_f);
        CHECK(merged.at(Value::create("b"s))->as<frst::Int>() == 2_f);
        CHECK(merged.at(Value::create("c"s))->as<frst::Int>() == 3_f);
        CHECK(merged.at(Value::create("d"s))->as<frst::Int>() == 4_f);
        CHECK(merged.at(Value::create("e"s))->as<frst::Int>() == 5_f);
        CHECK(merged.at(Value::create("f"s))->as<frst::Int>() == 6_f);
        CHECK(merged.at(Value::create("g"s))->as<frst::Int>() == 7_f);
        CHECK(merged.at(Value::create("h"s))->as<frst::Int>() == 8_f);
        CHECK(merged.at(Value::create("i"s))->as<frst::Int>() == 9_f);
    }

    SECTION("MAP + MAP (RHS shorter, LHS tail append)")
    {
        auto lhs = Value::create(frst::Map{
            {Value::create("m"s), Value::create(10_f)},
            {Value::create("n"s), Value::create(11_f)},
            {Value::create("o"s), Value::create(12_f)},
            {Value::create("p"s), Value::create(13_f)},
        });
        auto rhs = Value::create(frst::Map{
            {Value::create("a"s), Value::create(1_f)},
            {Value::create("b"s), Value::create(2_f)},
        });

        auto res = Value::add(lhs, rhs);
        const auto merged_opt = res->get<frst::Map>();
        REQUIRE(merged_opt.has_value());
        const auto& merged = merged_opt.value();

        CHECK(merged.size() == 6);
        CHECK(merged.at(Value::create("a"s))->as<frst::Int>() == 1_f);
        CHECK(merged.at(Value::create("b"s))->as<frst::Int>() == 2_f);
        CHECK(merged.at(Value::create("m"s))->as<frst::Int>() == 10_f);
        CHECK(merged.at(Value::create("n"s))->as<frst::Int>() == 11_f);
        CHECK(merged.at(Value::create("o"s))->as<frst::Int>() == 12_f);
        CHECK(merged.at(Value::create("p"s))->as<frst::Int>() == 13_f);
    }

    SECTION("MAP + MAP (Large maps, few collisions)")
    {
        frst::Map lhs_map;
        frst::Map rhs_map;

        // 50 keys on lhs: k000..k049
        for (frst::Int i = 0; i < 50; ++i)
        {
            const auto key =
                Value::create("k"
                              + (i < 10 ? "00"s : (i < 100 ? "0"s : ""s))
                              + std::to_string(i));
            lhs_map.emplace(key, Value::create(i));
        }

        // 50 keys on rhs: k040..k089 (10 collisions with lhs: k040..k049)
        for (frst::Int i = 40; i < 90; ++i)
        {
            const auto key =
                Value::create("k"
                              + (i < 10 ? "00"s : (i < 100 ? "0"s : ""s))
                              + std::to_string(i));
            rhs_map.emplace(key, Value::create(1000_f + i));
        }

        auto lhs = Value::create(std::move(lhs_map));
        auto rhs = Value::create(std::move(rhs_map));

        auto res = Value::add(lhs, rhs);
        const auto merged_opt = res->get<frst::Map>();
        REQUIRE(merged_opt.has_value());
        const auto& merged = merged_opt.value();

        CHECK(merged.size() == 90);

        // From lhs-only range.
        CHECK(merged.at(Value::create("k000"s))->as<frst::Int>() == 0_f);
        CHECK(merged.at(Value::create("k039"s))->as<frst::Int>() == 39_f);

        // Collisions should use rhs values.
        CHECK(merged.at(Value::create("k040"s))->as<frst::Int>() == 1040_f);
        CHECK(merged.at(Value::create("k049"s))->as<frst::Int>() == 1049_f);

        // From rhs-only tail.
        CHECK(merged.at(Value::create("k050"s))->as<frst::Int>() == 1050_f);
        CHECK(merged.at(Value::create("k089"s))->as<frst::Int>() == 1089_f);
    }

    SECTION("MAP + MAP (Equivalent float keys: +0.0 and -0.0)")
    {
        auto lhs = Value::create(frst::Map{
            {Value::create(0.0), Value::create(1_f)},
        });
        auto rhs = Value::create(frst::Map{
            {Value::create(-0.0), Value::create(2_f)},
        });

        auto res = Value::add(lhs, rhs);
        const auto merged_opt = res->get<frst::Map>();
        REQUIRE(merged_opt.has_value());
        const auto& merged = merged_opt.value();

        CHECK(merged.size() == 1);

        const auto by_pos_zero = merged.find(Value::create(0.0));
        const auto by_neg_zero = merged.find(Value::create(-0.0));
        REQUIRE(by_pos_zero != merged.end());
        REQUIRE(by_neg_zero != merged.end());
        CHECK(by_pos_zero->second->as<frst::Int>() == 2_f);
        CHECK(by_neg_zero->second->as<frst::Int>() == 2_f);
    }
}

TEST_CASE("Add All Permutations")
{
    auto Null = Value::null();
    auto Int = Value::create(42_f);
    auto Float = Value::create(3.14);
    auto Bool = Value::create(true);
    auto String = Value::create("Hello!"s);
    auto Array =
        Value::create(frst::Array{Value::create(64.314), Value::create(true)});
    auto Map = Value::create(frst::Map{
        {
            Value::create("foo"s),
            Value::create(500_f),
        },
        {
            Value::create("bar"s),
            Value::create(100.42),
        },
    });
    auto Function =
        Value::create(frst::Function{std::make_shared<Dummy_Callable>()});

#define OP_CHAR +
#define OP_VERB add
#define OP_METHOD add

    INCOMPAT(Null, Null)
    INCOMPAT(Null, Int)
    INCOMPAT(Null, Float)
    INCOMPAT(Null, Bool)
    INCOMPAT(Null, String)
    INCOMPAT(Null, Array)
    INCOMPAT(Null, Map)
    INCOMPAT(Int, Null)
    COMPAT(Int, Int)
    COMPAT(Int, Float)
    INCOMPAT(Int, Bool)
    INCOMPAT(Int, String)
    INCOMPAT(Int, Array)
    INCOMPAT(Int, Map)
    INCOMPAT(Float, Null)
    COMPAT(Float, Int)
    COMPAT(Float, Float)
    INCOMPAT(Float, Bool)
    INCOMPAT(Float, String)
    INCOMPAT(Float, Array)
    INCOMPAT(Float, Map)
    INCOMPAT(Bool, Null)
    INCOMPAT(Bool, Int)
    INCOMPAT(Bool, Float)
    INCOMPAT(Bool, Bool)
    INCOMPAT(Bool, String)
    INCOMPAT(Bool, Array)
    INCOMPAT(Bool, Map)
    INCOMPAT(String, Null)
    INCOMPAT(String, Int)
    INCOMPAT(String, Float)
    INCOMPAT(String, Bool)
    COMPAT(String, String)
    INCOMPAT(String, Array)
    INCOMPAT(String, Map)
    INCOMPAT(Array, Null)
    INCOMPAT(Array, Int)
    INCOMPAT(Array, Float)
    INCOMPAT(Array, Bool)
    INCOMPAT(Array, String)
    COMPAT(Array, Array)
    INCOMPAT(Array, Map)
    INCOMPAT(Map, Null)
    INCOMPAT(Map, Int)
    INCOMPAT(Map, Float)
    INCOMPAT(Map, Bool)
    INCOMPAT(Map, String)
    INCOMPAT(Map, Array)
    COMPAT(Map, Map)
    INCOMPAT(Null, Function)
    INCOMPAT(Int, Function)
    INCOMPAT(Float, Function)
    INCOMPAT(Bool, Function)
    INCOMPAT(String, Function)
    INCOMPAT(Array, Function)
    INCOMPAT(Map, Function)
    INCOMPAT(Function, Null)
    INCOMPAT(Function, Int)
    INCOMPAT(Function, Float)
    INCOMPAT(Function, Bool)
    INCOMPAT(Function, String)
    INCOMPAT(Function, Array)
    INCOMPAT(Function, Map)
    INCOMPAT(Function, Function)
}
