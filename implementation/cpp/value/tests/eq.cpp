#include <catch2/catch_all.hpp>

#include "op-test-macros.hpp"

#include <memory>

#include <frost/testing/stringmaker-specializations.hpp>
#include <frost/testing/dummy-callable.hpp>

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

TEST_CASE("Null Equality")
{
    auto null1 = Value::null();
    auto null2 = Value::null();

    SECTION("Identity Equals")
    {
        CHECK(Value::equal(null1, null1)->get<frst::Bool>().value());
    }

    SECTION("Diff Identity Equals")
    {
        CHECK(Value::equal(null1, null2)->get<frst::Bool>().value());
    }
}

TEST_CASE("Int Equality")
{
    auto int1 = Value::create(42_f);
    auto int2 = Value::create(42_f);
    auto int3 = Value::create(81_f);

    SECTION("Identity Equals")
    {
        CHECK(Value::equal(int1, int1)->get<frst::Bool>().value());
    }

    SECTION("Diff Identity Equals")
    {
        CHECK(Value::equal(int1, int2)->get<frst::Bool>().value());
    }

    SECTION("Unequal")
    {
        CHECK_FALSE(Value::equal(int1, int3)->get<frst::Bool>().value());
    }
}

TEST_CASE("Float Equality")
{
    auto flt1 = Value::create(3.14);
    auto flt2 = Value::create(3.14);
    auto flt3 = Value::create(2.17);

    SECTION("Identity Equals")
    {
        CHECK(Value::equal(flt1, flt1)->get<frst::Bool>().value());
    }

    SECTION("Diff Identity Equals")
    {
        CHECK(Value::equal(flt1, flt2)->get<frst::Bool>().value());
    }

    SECTION("Unequal")
    {
        CHECK_FALSE(Value::equal(flt1, flt3)->get<frst::Bool>().value());
    }
}

TEST_CASE("Bool Equality")
{
    auto b1 = Value::create(true);
    auto b2 = Value::create(true);
    auto b3 = Value::create(false);

    SECTION("Identity Equals")
    {
        CHECK(Value::equal(b1, b1)->get<frst::Bool>().value());
    }

    SECTION("Diff Identity Equals")
    {
        CHECK(Value::equal(b1, b2)->get<frst::Bool>().value());
    }

    SECTION("Unequal")
    {
        CHECK_FALSE(Value::equal(b1, b3)->get<frst::Bool>().value());
    }
}

TEST_CASE("String Equality")
{
    auto str1 = Value::create("foo"s);
    auto str2 = Value::create("foo"s);
    auto str3 = Value::create("bar"s);

    SECTION("Identity Equals")
    {
        CHECK(Value::equal(str1, str1)->get<frst::Bool>().value());
    }

    SECTION("Diff Identity Equals")
    {
        CHECK(Value::equal(str1, str2)->get<frst::Bool>().value());
    }

    SECTION("Unequal")
    {
        CHECK_FALSE(Value::equal(str1, str3)->get<frst::Bool>().value());
    }
}

TEST_CASE("Array Equality")
{
    auto arr1 = Value::create(frst::Array{});
    auto arr2 = Value::create(frst::Array{});

    SECTION("Equals")
    {
        CHECK(Value::equal(arr1, arr1)->get<frst::Bool>().value());
    }

    SECTION("Unequal")
    {
        CHECK_FALSE(Value::equal(arr1, arr2)->get<frst::Bool>().value());
    }
}

TEST_CASE("Map Equality")
{
    auto map1 = Value::create(frst::Map{});
    auto map2 = Value::create(frst::Map{});

    SECTION("Equals")
    {
        CHECK(Value::equal(map1, map1)->get<frst::Bool>().value());
    }

    SECTION("Unequal")
    {
        CHECK_FALSE(Value::equal(map1, map2)->get<frst::Bool>().value());
    }
}

TEST_CASE("Function Equality")
{
    auto fn_ptr = std::make_shared<Dummy_Callable>();
    auto fn1 = Value::create(frst::Function{fn_ptr});
    auto fn2 = Value::create(frst::Function{fn_ptr});
    auto fn3 =
        Value::create(frst::Function{std::make_shared<Dummy_Callable>()});

    CHECK(Value::equal(fn1, fn2)->get<frst::Bool>().value());
    CHECK_FALSE(Value::equal(fn1, fn3)->get<frst::Bool>().value());
}

TEST_CASE("Equals Compare All Permutations")
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

#define OP_CHAR ==
#define OP_VERB compare
#define OP_METHOD equal

#define ID_EQUAL_TEST(T1, T2)                                                  \
    SECTION("Id Equality: "s                                                   \
            + OP_TEST_STRINGIZE(T1)                                            \
            + " and "                                                          \
            + OP_TEST_STRINGIZE(T2))                                           \
    {                                                                          \
        if constexpr (std::is_same_v<frst::T1, frst::T2>)                      \
            CHECK(Value::OP_METHOD(T1, T2)->get<frst::Bool>().value());        \
        else                                                                   \
            CHECK_FALSE(Value::OP_METHOD(T1, T2)->get<frst::Bool>().value());  \
    }

    // any types can be compared for equality
    COMPAT(Null, Null)
    ID_EQUAL_TEST(Null, Null)
    COMPAT(Null, Int)
    ID_EQUAL_TEST(Null, Int)
    COMPAT(Null, Float)
    ID_EQUAL_TEST(Null, Float)
    COMPAT(Null, Bool)
    ID_EQUAL_TEST(Null, Bool)
    COMPAT(Null, String)
    ID_EQUAL_TEST(Null, String)
    COMPAT(Null, Array)
    ID_EQUAL_TEST(Null, Array)
    COMPAT(Null, Map)
    ID_EQUAL_TEST(Null, Map)
    COMPAT(Int, Null)
    ID_EQUAL_TEST(Int, Null)
    COMPAT(Int, Int)
    ID_EQUAL_TEST(Int, Int)
    COMPAT(Int, Float)
    ID_EQUAL_TEST(Int, Float)
    COMPAT(Int, Bool)
    ID_EQUAL_TEST(Int, Bool)
    COMPAT(Int, String)
    ID_EQUAL_TEST(Int, String)
    COMPAT(Int, Array)
    ID_EQUAL_TEST(Int, Array)
    COMPAT(Int, Map)
    ID_EQUAL_TEST(Int, Map)
    COMPAT(Float, Null)
    ID_EQUAL_TEST(Float, Null)
    COMPAT(Float, Int)
    ID_EQUAL_TEST(Float, Int)
    COMPAT(Float, Float)
    ID_EQUAL_TEST(Float, Float)
    COMPAT(Float, Bool)
    ID_EQUAL_TEST(Float, Bool)
    COMPAT(Float, String)
    ID_EQUAL_TEST(Float, String)
    COMPAT(Float, Array)
    ID_EQUAL_TEST(Float, Array)
    COMPAT(Float, Map)
    ID_EQUAL_TEST(Float, Map)
    COMPAT(Bool, Null)
    ID_EQUAL_TEST(Bool, Null)
    COMPAT(Bool, Int)
    ID_EQUAL_TEST(Bool, Int)
    COMPAT(Bool, Float)
    ID_EQUAL_TEST(Bool, Float)
    COMPAT(Bool, Bool)
    ID_EQUAL_TEST(Bool, Bool)
    COMPAT(Bool, String)
    ID_EQUAL_TEST(Bool, String)
    COMPAT(Bool, Array)
    ID_EQUAL_TEST(Bool, Array)
    COMPAT(Bool, Map)
    ID_EQUAL_TEST(Bool, Map)
    COMPAT(String, Null)
    ID_EQUAL_TEST(String, Null)
    COMPAT(String, Int)
    ID_EQUAL_TEST(String, Int)
    COMPAT(String, Float)
    ID_EQUAL_TEST(String, Float)
    COMPAT(String, Bool)
    ID_EQUAL_TEST(String, Bool)
    COMPAT(String, String)
    ID_EQUAL_TEST(String, String)
    COMPAT(String, Array)
    ID_EQUAL_TEST(String, Array)
    COMPAT(String, Map)
    ID_EQUAL_TEST(String, Map)
    COMPAT(Array, Null)
    ID_EQUAL_TEST(Array, Null)
    COMPAT(Array, Int)
    ID_EQUAL_TEST(Array, Int)
    COMPAT(Array, Float)
    ID_EQUAL_TEST(Array, Float)
    COMPAT(Array, Bool)
    ID_EQUAL_TEST(Array, Bool)
    COMPAT(Array, String)
    ID_EQUAL_TEST(Array, String)
    COMPAT(Array, Array)
    ID_EQUAL_TEST(Array, Array)
    COMPAT(Array, Map)
    ID_EQUAL_TEST(Array, Map)
    COMPAT(Map, Null)
    ID_EQUAL_TEST(Map, Null)
    COMPAT(Map, Int)
    ID_EQUAL_TEST(Map, Int)
    COMPAT(Map, Float)
    ID_EQUAL_TEST(Map, Float)
    COMPAT(Map, Bool)
    ID_EQUAL_TEST(Map, Bool)
    COMPAT(Map, String)
    ID_EQUAL_TEST(Map, String)
    COMPAT(Map, Array)
    ID_EQUAL_TEST(Map, Array)
    COMPAT(Map, Map)
    ID_EQUAL_TEST(Map, Map)
    COMPAT(Null, Function)
    ID_EQUAL_TEST(Null, Function)
    COMPAT(Int, Function)
    ID_EQUAL_TEST(Int, Function)
    COMPAT(Float, Function)
    ID_EQUAL_TEST(Float, Function)
    COMPAT(Bool, Function)
    ID_EQUAL_TEST(Bool, Function)
    COMPAT(String, Function)
    ID_EQUAL_TEST(String, Function)
    COMPAT(Array, Function)
    ID_EQUAL_TEST(Array, Function)
    COMPAT(Map, Function)
    ID_EQUAL_TEST(Map, Function)
    COMPAT(Function, Null)
    ID_EQUAL_TEST(Function, Null)
    COMPAT(Function, Int)
    ID_EQUAL_TEST(Function, Int)
    COMPAT(Function, Float)
    ID_EQUAL_TEST(Function, Float)
    COMPAT(Function, Bool)
    ID_EQUAL_TEST(Function, Bool)
    COMPAT(Function, String)
    ID_EQUAL_TEST(Function, String)
    COMPAT(Function, Array)
    ID_EQUAL_TEST(Function, Array)
    COMPAT(Function, Map)
    ID_EQUAL_TEST(Function, Map)
    COMPAT(Function, Function)
    ID_EQUAL_TEST(Function, Function)
}
