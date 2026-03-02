#include <catch2/catch_all.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <frost/builtin.hpp>

using namespace frst;

using namespace std::literals;

using namespace Catch::Matchers;

TEST_CASE("Builtin values")
{
    Symbol_Table table;
    inject_builtins(table);
    auto values_val = table.lookup("values");
    REQUIRE(values_val->is<Function>());
    auto values = values_val->get<Function>().value();

    SECTION("Wrong Type")
    {
        std::vector<Value_Ptr> not_maps{
            Value::null(),
            Value::create(42_f),
            Value::create(3.14),
            Value::create(true),
            Value::create("Hello!"s),
            Value::create(
                frst::Array{Value::create(64.314), Value::create(true)}),
        };

        for (const auto& val : not_maps)
        {
            CHECK_THROWS_WITH(values->call({val}),
                              EndsWith(std::string{val->type_name()}));
        }
    }

    SECTION("Good")
    {
        auto map = Value::create(Map{
            {Value::create("hello"s), Value::create(42_f)},
            {Value::create("world"s), Value::create(10_f)},
        });

        auto res = values->call({map});

        CHECK(res->is<Array>());
        auto res_arr = res->get<Array>().value();
        CHECK(res_arr.size() == 2);
        CHECK(res_arr.at(0)->get<Int>().value_or(0) == 42_f);
        CHECK(res_arr.at(1)->get<Int>().value_or(0) == 10_f);
    }
}
