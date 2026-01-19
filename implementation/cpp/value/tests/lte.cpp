// AI-generated test additions by Codex (GPT-5).
#include <catch2/catch_all.hpp>

#include "catch2/catch_test_macros.hpp"
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

TEST_CASE("Numeric LTE")
{
    auto int1 = Value::create(3_f);
    auto int2 = Value::create(3_f);
    auto int3 = Value::create(42_f);
    auto int4 = Value::create(42_f);

    auto flt1 = Value::create(2.14);
    auto flt2 = Value::create(2.14);
    auto flt3 = Value::create(3.13);
    auto flt4 = Value::create(3.13);

    SECTION("Int Identity LTE")
    {
        CHECK(Value::less_than_or_equal(int1, int1)->get<frst::Bool>().value());
    }

    SECTION("Int LTE")
    {
        CHECK(Value::less_than_or_equal(int1, int2)->get<frst::Bool>().value());
        CHECK(Value::less_than_or_equal(int1, int3)->get<frst::Bool>().value());
        CHECK_FALSE(
            Value::less_than_or_equal(int3, int1)->get<frst::Bool>().value());
        CHECK(Value::less_than_or_equal(int3, int4)->get<frst::Bool>().value());
    }

    SECTION("Float Identity LTE")
    {
        CHECK(Value::less_than_or_equal(flt1, flt1)->get<frst::Bool>().value());
    }

    SECTION("Float LTE")
    {
        CHECK(Value::less_than_or_equal(flt1, flt2)->get<frst::Bool>().value());
        CHECK(Value::less_than_or_equal(flt1, flt3)->get<frst::Bool>().value());
        CHECK_FALSE(
            Value::less_than_or_equal(flt3, flt1)->get<frst::Bool>().value());
        CHECK(Value::less_than_or_equal(flt3, flt4)->get<frst::Bool>().value());
    }

    SECTION("Mixed LTE")
    {
        CHECK(Value::less_than_or_equal(flt1, int1)->get<frst::Bool>().value());
        CHECK(Value::less_than_or_equal(flt1, int3)->get<frst::Bool>().value());
        CHECK_FALSE(
            Value::less_than_or_equal(flt3, int1)->get<frst::Bool>().value());
        CHECK(Value::less_than_or_equal(flt3, int3)->get<frst::Bool>().value());

        CHECK_FALSE(
            Value::less_than_or_equal(int1, flt1)->get<frst::Bool>().value());
        CHECK_FALSE(
            Value::less_than_or_equal(int3, flt1)->get<frst::Bool>().value());
        CHECK(Value::less_than_or_equal(int1, flt3)->get<frst::Bool>().value());
        CHECK_FALSE(
            Value::less_than_or_equal(int3, flt3)->get<frst::Bool>().value());
    }
}

TEST_CASE("String Lexicographical LTE")
{
    auto foo = Value::create("foo"s);
    auto bar = Value::create("bar"s);
    auto foobar = Value::create("foobar"s);

    CHECK(Value::less_than_or_equal(bar, foo)->get<frst::Bool>().value());
    CHECK_FALSE(Value::less_than_or_equal(foo, bar)->get<frst::Bool>().value());
    CHECK(Value::less_than_or_equal(foo, foo)->get<frst::Bool>().value());
    CHECK(Value::less_than_or_equal(foo, foobar)->get<frst::Bool>().value());
}

TEST_CASE("Less Than Or Equal Compare All Permutations")
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

#define OP_CHAR <=
#define OP_VERB compare
#define OP_METHOD less_than_or_equal

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
