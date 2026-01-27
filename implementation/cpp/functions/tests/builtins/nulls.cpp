// AI-generated test by Codex (GPT-5).
// Signed: Codex (GPT-5).
#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <vector>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <frost/builtin.hpp>

using namespace frst;

using namespace std::literals;
using Catch::Matchers::ContainsSubstring;
using Catch::Matchers::EndsWith;

TEST_CASE("Builtin nulls")
{
    Symbol_Table table;
    inject_builtins(table);
    auto nulls_val = table.lookup("nulls");
    REQUIRE(nulls_val->is<Function>());
    auto nulls = nulls_val->get<Function>().value();

    SECTION("Injected")
    {
        CHECK(nulls_val->is<Function>());
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(nulls->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(nulls->call({Value::null(), Value::null()}),
                          ContainsSubstring("too many arguments"));
    }

    SECTION("Type errors")
    {
        auto bad_vals = std::vector<Value_Ptr>{
            Value::null(),          Value::create(3.14),
            Value::create(true),    Value::create("nope"s),
            Value::create(Array{}), Value::create(Map{}),
        };

        for (const auto& val : bad_vals)
        {
            CHECK_THROWS_WITH(nulls->call({val}),
                              ContainsSubstring("Function nulls requires"));
            CHECK_THROWS_WITH(nulls->call({val}),
                              ContainsSubstring("argument 1"));
            CHECK_THROWS_WITH(nulls->call({val}),
                              EndsWith(std::string{val->type_name()}));
        }
    }

    SECTION("Negative counts fail")
    {
        CHECK_THROWS_WITH(nulls->call({Value::create(-1_f)}),
                          ContainsSubstring("requires positive argument"));
        CHECK_THROWS_WITH(nulls->call({Value::create(-1_f)}),
                          ContainsSubstring("got -1"));
    }

    SECTION("Zero returns empty array")
    {
        auto res = nulls->call({Value::create(0_f)});
        REQUIRE(res->is<Array>());
        CHECK(res->get<Array>().value().empty());
    }

    SECTION("All elements are the null singleton")
    {
        auto res = nulls->call({Value::create(3_f)});
        REQUIRE(res->is<Array>());
        const auto& arr = res->raw_get<Array>();
        REQUIRE(arr.size() == 3);
        for (const auto& elem : arr)
        {
            CHECK(elem == Value::null());
        }
    }
}
