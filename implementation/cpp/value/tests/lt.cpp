#include <catch2/catch_all.hpp>

#include "catch-stringmaker-specializations.hpp"
#include "catch2/catch_test_macros.hpp"
#include "op-test-macros.hpp"

#include <frost/value.hpp>

using namespace std::literals;
using namespace frst::literals;
using frst::Value, frst::Value_Ptr;

TEST_CASE("Numeric LT")
{
    auto int1 = Value::create(3_f);
    auto int2 = Value::create(3_f);
    auto int3 = Value::create(42_f);
    auto int4 = Value::create(42_f);

    auto flt1 = Value::create(2.14);
    auto flt2 = Value::create(2.14);
    auto flt3 = Value::create(3.13);
    auto flt4 = Value::create(3.13);

    SECTION("Int Identity LT")
    {
        CHECK_FALSE(Value::less_than(int1, int1)->get<frst::Bool>().value());
    }

    SECTION("Int LT")
    {
        CHECK(Value::less_than(int1, int3)->get<frst::Bool>().value());
        CHECK_FALSE(Value::less_than(int3, int1)->get<frst::Bool>().value());
    }

    SECTION("Float Identity LT")
    {
        CHECK_FALSE(Value::less_than(flt1, flt1)->get<frst::Bool>().value());
    }

    SECTION("Float LT")
    {
        CHECK(Value::less_than(flt1, flt3)->get<frst::Bool>().value());
        CHECK_FALSE(Value::less_than(flt3, flt1)->get<frst::Bool>().value());
    }

    SECTION("Mixed LT")
    {
        CHECK(Value::less_than(flt1, int1)->get<frst::Bool>().value());
        CHECK(Value::less_than(flt1, int3)->get<frst::Bool>().value());
        CHECK_FALSE(Value::less_than(flt3, int1)->get<frst::Bool>().value());
        CHECK(Value::less_than(flt3, int3)->get<frst::Bool>().value());

        CHECK_FALSE(Value::less_than(int1, flt1)->get<frst::Bool>().value());
        CHECK_FALSE(Value::less_than(int3, flt1)->get<frst::Bool>().value());
        CHECK(Value::less_than(int1, flt3)->get<frst::Bool>().value());
        CHECK_FALSE(Value::less_than(int3, flt3)->get<frst::Bool>().value());
    }
}

TEST_CASE("String Lexicographical LT")
{
    auto foo = Value::create("foo"s);
    auto bar = Value::create("bar"s);
    auto foobar = Value::create("foobar"s);

    CHECK(Value::less_than(bar, foo)->get<frst::Bool>().value());
    CHECK_FALSE(Value::less_than(foo, bar)->get<frst::Bool>().value());
    CHECK(Value::less_than(foo, foobar)->get<frst::Bool>().value());
}

TEST_CASE("Equals Compare All Permutations")
{
    auto Null = Value::create();
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

#define OP_CHAR <
#define OP_VERB compare
#define OP_METHOD less_than

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
    INCOMPAT(Array, Array)
    INCOMPAT(Array, Map)
    INCOMPAT(Map, Null)
    INCOMPAT(Map, Int)
    INCOMPAT(Map, Float)
    INCOMPAT(Map, Bool)
    INCOMPAT(Map, String)
    INCOMPAT(Map, Array)
    INCOMPAT(Map, Map)
}
