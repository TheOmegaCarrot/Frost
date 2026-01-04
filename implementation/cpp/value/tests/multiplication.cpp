#include <catch2/catch_all.hpp>
#include <catch2/catch_test_macros.hpp>

#include <fmt/std.h>

#include <frost/value.hpp>

using namespace std::literals;
using namespace frst::literals;
using frst::Value, frst::Value_Ptr;

namespace Catch
{
template <typename T>
struct StringMaker<std::optional<T>>
{
    static std::string convert(const std::optional<T>& value)
    {
        return fmt::format("{}", value);
    }
};
} // namespace Catch

TEST_CASE("Numeric Multiplication")
{
    auto int1 = Value::create(42_f);
    auto int2 = Value::create(81_f);
    auto flt1 = Value::create(3.14);
    auto flt2 = Value::create(2.17);

    SECTION("INT * INT")
    {
        auto res = Value::multiply(int1, int2);
        REQUIRE(res->is<frst::Int>());
        CHECK(res->get<frst::Int>().value() == 42_f * 81_f);
    }

    SECTION("FLOAT * FLOAT")
    {
        auto res = Value::multiply(flt1, flt2);
        REQUIRE(res->is<frst::Float>());
        CHECK(res->get<frst::Float>().value() == 3.14 * 2.17);
    }

    SECTION("INT * FLOAT")
    {
        auto res = Value::multiply(int1, flt1);
        REQUIRE(res->is<frst::Float>());
        CHECK(res->get<frst::Float>().value() == 42_f * 3.14);
    }

    SECTION("FLOAT * INT")
    {
        auto res = Value::multiply(flt2, int2);
        REQUIRE(res->is<frst::Float>());
        CHECK(res->get<frst::Float>().value() == 2.17 * 81_f);
    }
}

TEST_CASE("Multiply All Permutations")
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

#define OP_CHAR *
#define OP_VERB multiply

#include "op-test-macros.hpp"

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
}
