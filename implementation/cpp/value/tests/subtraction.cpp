#include <catch2/catch_all.hpp>

#include "op-test-macros.hpp"

#include <memory>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/value.hpp>

using namespace std::literals;
using namespace frst::literals;
using frst::Value, frst::Value_Ptr;

namespace
{
// AI-generated test additions by Codex (GPT-5).
// Signed: Codex (GPT-5).
struct Dummy_Function final : frst::Callable
{
    frst::Value_Ptr call(std::span<const frst::Value_Ptr>) const override
    {
        return Value::null();
    }

    std::string debug_dump() const override
    {
        return "<dummy>";
    }
};
} // namespace

TEST_CASE("Numeric Subtraction")
{
    auto int1 = Value::create(42_f);
    auto int2 = Value::create(81_f);
    auto flt1 = Value::create(3.14);
    auto flt2 = Value::create(2.17);

    SECTION("INT - INT")
    {
        auto res = Value::subtract(int1, int2);
        REQUIRE(res->is<frst::Int>());
        CHECK(res->get<frst::Int>().value() == 42_f - 81_f);
    }

    SECTION("FLOAT - FLOAT")
    {
        auto res = Value::subtract(flt1, flt2);
        REQUIRE(res->is<frst::Float>());
        CHECK(res->get<frst::Float>().value() == 3.14 - 2.17);
    }

    SECTION("INT - FLOAT")
    {
        auto res = Value::subtract(int1, flt1);
        REQUIRE(res->is<frst::Float>());
        CHECK(res->get<frst::Float>().value() == 42_f - 3.14);
    }

    SECTION("FLOAT - INT")
    {
        auto res = Value::subtract(flt2, int2);
        REQUIRE(res->is<frst::Float>());
        CHECK(res->get<frst::Float>().value() == 2.17 - 81_f);
    }
}

TEST_CASE("Subtract All Permutations")
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
        Value::create(frst::Function{std::make_shared<Dummy_Function>()});

#define OP_CHAR -
#define OP_VERB subtract
#define OP_METHOD subtract

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
    INCOMPAT(String, String)
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
