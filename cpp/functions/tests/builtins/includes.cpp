#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <frost/builtin.hpp>

using namespace frst;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;

TEST_CASE("Builtin includes")
{
    Symbol_Table table;
    inject_builtins(table);
    auto includes_val = table.lookup("includes");
    REQUIRE(includes_val->is<Function>());
    auto includes = includes_val->get<Function>().value();

    SECTION("Injected")
    {
        CHECK(includes_val->is<Function>());
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(includes->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(includes->call({Value::create(Array{})}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(includes->call({Value::create(Array{}), Value::null(),
                                          Value::null()}),
                          ContainsSubstring("too many arguments"));
    }

    SECTION("First argument must be Array")
    {
        CHECK_THROWS_WITH(includes->call({Value::create(1_f), Value::null()}),
                          ContainsSubstring("Function includes"));
        CHECK_THROWS_WITH(includes->call({Value::create(1_f), Value::null()}),
                          ContainsSubstring("Array"));
    }

    SECTION("Finds present elements")
    {
        auto arr = Value::create(Array{
            Value::create(1_f),
            Value::create("hello"s),
            Value::create(true),
            Value::null(),
        });

        CHECK(includes->call({arr, Value::create(1_f)})->truthy());
        CHECK(includes->call({arr, Value::create("hello"s)})->truthy());
        CHECK(includes->call({arr, Value::create(true)})->truthy());
        CHECK(includes->call({arr, Value::null()})->truthy());
    }

    SECTION("Missing elements return false")
    {
        auto arr = Value::create(Array{
            Value::create(1_f),
            Value::create("hello"s),
        });

        CHECK_FALSE(includes->call({arr, Value::create(2_f)})->truthy());
        CHECK_FALSE(includes->call({arr, Value::create("world"s)})->truthy());
        CHECK_FALSE(includes->call({arr, Value::null()})->truthy());
    }

    SECTION("Empty array contains nothing")
    {
        auto empty = Value::create(Array{});

        CHECK_FALSE(includes->call({empty, Value::create(0_f)})->truthy());
        CHECK_FALSE(includes->call({empty, Value::null()})->truthy());
    }

    SECTION("Uses value equality, not identity")
    {
        auto arr = Value::create(Array{
            Value::create(Array{Value::create(1_f), Value::create(2_f)}),
        });

        auto needle =
            Value::create(Array{Value::create(1_f), Value::create(2_f)});

        CHECK(includes->call({arr, needle})->truthy());
    }
}
