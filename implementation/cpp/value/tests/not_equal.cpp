// AI-generated test additions by Codex (GPT-5).
#include <catch2/catch_all.hpp>

#include "op-test-macros.hpp"

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/value.hpp>

using namespace std::literals;
using namespace frst::literals;
using frst::Value, frst::Value_Ptr;

TEST_CASE("Null Not Equal")
{
    auto null1 = Value::null();
    auto null2 = Value::null();

    SECTION("Identity Equals")
    {
        CHECK_FALSE(Value::not_equal(null1, null1)->get<frst::Bool>().value());
    }

    SECTION("Diff Identity Equals")
    {
        CHECK_FALSE(Value::not_equal(null1, null2)->get<frst::Bool>().value());
    }
}

TEST_CASE("Int Not Equal")
{
    auto int1 = Value::create(42_f);
    auto int2 = Value::create(42_f);
    auto int3 = Value::create(81_f);

    SECTION("Identity Equals")
    {
        CHECK_FALSE(Value::not_equal(int1, int1)->get<frst::Bool>().value());
    }

    SECTION("Diff Identity Equals")
    {
        CHECK_FALSE(Value::not_equal(int1, int2)->get<frst::Bool>().value());
    }

    SECTION("Unequal")
    {
        CHECK(Value::not_equal(int1, int3)->get<frst::Bool>().value());
    }
}

TEST_CASE("Float Not Equal")
{
    auto flt1 = Value::create(3.14);
    auto flt2 = Value::create(3.14);
    auto flt3 = Value::create(2.17);

    SECTION("Identity Equals")
    {
        CHECK_FALSE(Value::not_equal(flt1, flt1)->get<frst::Bool>().value());
    }

    SECTION("Diff Identity Equals")
    {
        CHECK_FALSE(Value::not_equal(flt1, flt2)->get<frst::Bool>().value());
    }

    SECTION("Unequal")
    {
        CHECK(Value::not_equal(flt1, flt3)->get<frst::Bool>().value());
    }
}

TEST_CASE("Bool Not Equal")
{
    auto b1 = Value::create(true);
    auto b2 = Value::create(true);
    auto b3 = Value::create(false);

    SECTION("Identity Equals")
    {
        CHECK_FALSE(Value::not_equal(b1, b1)->get<frst::Bool>().value());
    }

    SECTION("Diff Identity Equals")
    {
        CHECK_FALSE(Value::not_equal(b1, b2)->get<frst::Bool>().value());
    }

    SECTION("Unequal")
    {
        CHECK(Value::not_equal(b1, b3)->get<frst::Bool>().value());
    }
}

TEST_CASE("String Not Equal")
{
    auto str1 = Value::create("foo"s);
    auto str2 = Value::create("foo"s);
    auto str3 = Value::create("bar"s);

    SECTION("Identity Equals")
    {
        CHECK_FALSE(Value::not_equal(str1, str1)->get<frst::Bool>().value());
    }

    SECTION("Diff Identity Equals")
    {
        CHECK_FALSE(Value::not_equal(str1, str2)->get<frst::Bool>().value());
    }

    SECTION("Unequal")
    {
        CHECK(Value::not_equal(str1, str3)->get<frst::Bool>().value());
    }
}

TEST_CASE("Array Not Equal")
{
    auto arr1 = Value::create(frst::Array{});
    auto arr2 = Value::create(frst::Array{});

    SECTION("Equals")
    {
        CHECK_FALSE(Value::not_equal(arr1, arr1)->get<frst::Bool>().value());
    }

    SECTION("Unequal")
    {
        CHECK(Value::not_equal(arr1, arr2)->get<frst::Bool>().value());
    }
}

TEST_CASE("Map Not Equal")
{
    auto map1 = Value::create(frst::Map{});
    auto map2 = Value::create(frst::Map{});

    SECTION("Equals")
    {
        CHECK_FALSE(Value::not_equal(map1, map1)->get<frst::Bool>().value());
    }

    SECTION("Unequal")
    {
        CHECK(Value::not_equal(map1, map2)->get<frst::Bool>().value());
    }
}

TEST_CASE("Not Equal Compare All Permutations")
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

#define OP_CHAR !=
#define OP_VERB compare
#define OP_METHOD not_equal

#define ID_NOT_EQUAL_TEST(T1, T2)                                              \
    SECTION("Id Inequality: "s                                                 \
            + OP_TEST_STRINGIZE(T1)                                            \
            + " and "                                                          \
            + OP_TEST_STRINGIZE(T2))                                           \
    {                                                                          \
        if constexpr (std::is_same_v<frst::T1, frst::T2>)                      \
            CHECK_FALSE(Value::OP_METHOD(T1, T2)->get<frst::Bool>().value());  \
        else                                                                   \
            CHECK(Value::OP_METHOD(T1, T2)->get<frst::Bool>().value());        \
    }

    // any types can be compared for inequality
    COMPAT(Null, Null)
    ID_NOT_EQUAL_TEST(Null, Null)
    COMPAT(Null, Int)
    ID_NOT_EQUAL_TEST(Null, Int)
    COMPAT(Null, Float)
    ID_NOT_EQUAL_TEST(Null, Float)
    COMPAT(Null, Bool)
    ID_NOT_EQUAL_TEST(Null, Bool)
    COMPAT(Null, String)
    ID_NOT_EQUAL_TEST(Null, String)
    COMPAT(Null, Array)
    ID_NOT_EQUAL_TEST(Null, Array)
    COMPAT(Null, Map)
    ID_NOT_EQUAL_TEST(Null, Map)
    COMPAT(Int, Null)
    ID_NOT_EQUAL_TEST(Int, Null)
    COMPAT(Int, Int)
    ID_NOT_EQUAL_TEST(Int, Int)
    COMPAT(Int, Float)
    ID_NOT_EQUAL_TEST(Int, Float)
    COMPAT(Int, Bool)
    ID_NOT_EQUAL_TEST(Int, Bool)
    COMPAT(Int, String)
    ID_NOT_EQUAL_TEST(Int, String)
    COMPAT(Int, Array)
    ID_NOT_EQUAL_TEST(Int, Array)
    COMPAT(Int, Map)
    ID_NOT_EQUAL_TEST(Int, Map)
    COMPAT(Float, Null)
    ID_NOT_EQUAL_TEST(Float, Null)
    COMPAT(Float, Int)
    ID_NOT_EQUAL_TEST(Float, Int)
    COMPAT(Float, Float)
    ID_NOT_EQUAL_TEST(Float, Float)
    COMPAT(Float, Bool)
    ID_NOT_EQUAL_TEST(Float, Bool)
    COMPAT(Float, String)
    ID_NOT_EQUAL_TEST(Float, String)
    COMPAT(Float, Array)
    ID_NOT_EQUAL_TEST(Float, Array)
    COMPAT(Float, Map)
    ID_NOT_EQUAL_TEST(Float, Map)
    COMPAT(Bool, Null)
    ID_NOT_EQUAL_TEST(Bool, Null)
    COMPAT(Bool, Int)
    ID_NOT_EQUAL_TEST(Bool, Int)
    COMPAT(Bool, Float)
    ID_NOT_EQUAL_TEST(Bool, Float)
    COMPAT(Bool, Bool)
    ID_NOT_EQUAL_TEST(Bool, Bool)
    COMPAT(Bool, String)
    ID_NOT_EQUAL_TEST(Bool, String)
    COMPAT(Bool, Array)
    ID_NOT_EQUAL_TEST(Bool, Array)
    COMPAT(Bool, Map)
    ID_NOT_EQUAL_TEST(Bool, Map)
    COMPAT(String, Null)
    ID_NOT_EQUAL_TEST(String, Null)
    COMPAT(String, Int)
    ID_NOT_EQUAL_TEST(String, Int)
    COMPAT(String, Float)
    ID_NOT_EQUAL_TEST(String, Float)
    COMPAT(String, Bool)
    ID_NOT_EQUAL_TEST(String, Bool)
    COMPAT(String, String)
    ID_NOT_EQUAL_TEST(String, String)
    COMPAT(String, Array)
    ID_NOT_EQUAL_TEST(String, Array)
    COMPAT(String, Map)
    ID_NOT_EQUAL_TEST(String, Map)
    COMPAT(Array, Null)
    ID_NOT_EQUAL_TEST(Array, Null)
    COMPAT(Array, Int)
    ID_NOT_EQUAL_TEST(Array, Int)
    COMPAT(Array, Float)
    ID_NOT_EQUAL_TEST(Array, Float)
    COMPAT(Array, Bool)
    ID_NOT_EQUAL_TEST(Array, Bool)
    COMPAT(Array, String)
    ID_NOT_EQUAL_TEST(Array, String)
    COMPAT(Array, Array)
    ID_NOT_EQUAL_TEST(Array, Array)
    COMPAT(Array, Map)
    ID_NOT_EQUAL_TEST(Array, Map)
    COMPAT(Map, Null)
    ID_NOT_EQUAL_TEST(Map, Null)
    COMPAT(Map, Int)
    ID_NOT_EQUAL_TEST(Map, Int)
    COMPAT(Map, Float)
    ID_NOT_EQUAL_TEST(Map, Float)
    COMPAT(Map, Bool)
    ID_NOT_EQUAL_TEST(Map, Bool)
    COMPAT(Map, String)
    ID_NOT_EQUAL_TEST(Map, String)
    COMPAT(Map, Array)
    ID_NOT_EQUAL_TEST(Map, Array)
    COMPAT(Map, Map)
    ID_NOT_EQUAL_TEST(Map, Map)
}
