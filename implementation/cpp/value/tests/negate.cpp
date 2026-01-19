#include <catch2/catch_all.hpp>

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

TEST_CASE("Unary Negation")
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

    SECTION("Int")
    {
        CHECK(Int->negate()->get<frst::Int>() == -42_f);
    }

    SECTION("Float")
    {
        CHECK(Float->negate()->get<frst::Float>() == -3.14);
    }

#define INVALID(T)                                                             \
    SECTION(#T)                                                                \
    {                                                                          \
        CHECK_THROWS_WITH(T->negate(), "Invalid operand for unary - : " #T);   \
    }

    INVALID(Null)
    INVALID(Bool)
    INVALID(String)
    INVALID(Array)
    INVALID(Map)
    INVALID(Function)
}
