// AI-generated test additions by Codex (GPT-5).
#include <catch2/catch_all.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/value.hpp>

using namespace std::literals;
using namespace frst::literals;
using frst::Value, frst::Value_Ptr;

namespace
{
struct Logical_Case
{
    const char* name;
    Value_Ptr value;
    bool truthy;
};

std::vector<Logical_Case> logical_cases()
{
    return {
        {"null", Value::null(), false},
        {"false", Value::create(false), false},
        {"true", Value::create(true), true},
        {"int_zero", Value::create(0_f), true},
        {"float_zero", Value::create(0.0), true},
        {"empty_string", Value::create(""s), true},
        {"empty_array", Value::create(frst::Array{}), true},
        {"empty_map", Value::create(frst::Map{}), true},
    };
}
} // namespace

TEST_CASE("Logical Not")
{
    auto cases = logical_cases();

    for (const auto& item : cases)
    {
        DYNAMIC_SECTION("value=" << item.name)
        {
            auto res = item.value->logical_not();
            REQUIRE(res->is<frst::Bool>());
            CHECK(res->get<frst::Bool>().value() == !item.truthy);
        }
    }
}
