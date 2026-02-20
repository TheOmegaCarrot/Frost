#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/testing/dummy-callable.hpp>
#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <frost/builtin.hpp>

#include <vector>

using namespace frst;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;

TEST_CASE("Builtin has")
{
    Symbol_Table table;
    inject_builtins(table);
    auto has_val = table.lookup("has");
    REQUIRE(has_val->is<Function>());
    auto has = has_val->get<Function>().value();

    SECTION("Injected")
    {
        CHECK(has_val->is<Function>());
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(has->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(has->call({Value::create(Array{})}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(
            has->call(
                {Value::create(Array{}), Value::create(0_f), Value::null()}),
            ContainsSubstring("too many arguments"));
    }

    SECTION("First argument must be Array or Map")
    {
        auto fn = Value::create(
            Function{std::make_shared<frst::testing::Dummy_Callable>()});
        std::vector<Value_Ptr> bad_structures{
            Value::null(),
            Value::create(1_f),
            Value::create(3.14),
            Value::create(true),
            Value::create("hello"s),
            fn,
        };

        for (const auto& bad_structure : bad_structures)
        {
            CHECK_THROWS_WITH(
                has->call({bad_structure, Value::create(0_f)}),
                ContainsSubstring("Function has requires"));
            CHECK_THROWS_WITH(
                has->call({bad_structure, Value::create(0_f)}),
                ContainsSubstring("Array or Map"));
            CHECK_THROWS_WITH(
                has->call({bad_structure, Value::create(0_f)}),
                ContainsSubstring("structure"));
            CHECK_THROWS_WITH(
                has->call({bad_structure, Value::create(0_f)}),
                ContainsSubstring(std::string{bad_structure->type_name()}));
        }
    }

    SECTION("Map membership semantics")
    {
        auto key_str = Value::create("alpha"s);
        auto key_int = Value::create(7_f);
        auto key_float = Value::create(2.5);
        auto key_bool = Value::create(true);

        auto map = Value::create(Map{
            {key_str, Value::create(1_f)},
            {key_int, Value::create(2_f)},
            {key_float, Value::create(3_f)},
            {key_bool, Value::create(4_f)},
        });

        auto call_has = [&](const Value_Ptr& key) -> bool {
            auto result = has->call({map, key});
            REQUIRE(result->is<Bool>());
            return result->get<Bool>().value();
        };

        SECTION("Present keys return true")
        {
            CHECK(call_has(key_str));
            CHECK(call_has(key_int));
            CHECK(call_has(key_float));
            CHECK(call_has(key_bool));
        }

        SECTION("Missing keys return false")
        {
            CHECK_FALSE(call_has(Value::create("missing"s)));
            CHECK_FALSE(call_has(Value::create(8_f)));
            CHECK_FALSE(call_has(Value::create(2.6)));
            CHECK_FALSE(call_has(Value::create(false)));
        }

        SECTION("Null key is rejected")
        {
            CHECK_THROWS_WITH(
                has->call({map, Value::null()}),
                ContainsSubstring("Function has requires"));
            CHECK_THROWS_WITH(
                has->call({map, Value::null()}),
                ContainsSubstring("argument 2"));
            CHECK_THROWS_WITH(has->call({map, Value::null()}),
                              ContainsSubstring("Null"));
        }
    }

    SECTION("Array index semantics")
    {
        auto array = Value::create(Array{
            Value::create(10_f),
            Value::create(20_f),
            Value::create(30_f),
        });

        auto call_has = [&](Int index) -> bool {
            auto result = has->call({array, Value::create(index)});
            REQUIRE(result->is<Bool>());
            return result->get<Bool>().value();
        };

        SECTION("Valid indices return true")
        {
            CHECK(call_has(0));
            CHECK(call_has(2));
            CHECK(call_has(-1));
            CHECK(call_has(-3));
        }

        SECTION("Invalid indices return false")
        {
            CHECK_FALSE(call_has(3));
            CHECK_FALSE(call_has(-4));
            CHECK_FALSE(call_has(999));
            CHECK_FALSE(call_has(-999));
        }

        SECTION("Empty arrays have no valid indices")
        {
            auto empty = Value::create(Array{});

            auto call_has_empty = [&](Int index) -> bool {
                auto result = has->call({empty, Value::create(index)});
                REQUIRE(result->is<Bool>());
                return result->get<Bool>().value();
            };

            CHECK_FALSE(call_has_empty(0));
            CHECK_FALSE(call_has_empty(-1));
        }

        SECTION("Array requires Int as argument 2")
        {
            std::vector<Value_Ptr> non_int_indices{
                Value::create("0"s),
                Value::create(3.14),
                Value::create(true),
            };

            for (const auto& bad_index : non_int_indices)
            {
                CHECK_THROWS_WITH(
                    has->call({array, bad_index}),
                    ContainsSubstring(
                        "Function has with Array requires Int as argument 2"));
                CHECK_THROWS_WITH(
                    has->call({array, bad_index}),
                    ContainsSubstring(std::string{bad_index->type_name()}));
            }
        }
    }
}
