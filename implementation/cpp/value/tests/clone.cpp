#include <catch2/catch_test_macros.hpp>

#include <vector>

#include <frost/testing/dummy-callable.hpp>
#include <frost/value.hpp>

using namespace frst;
using namespace std::literals;
using namespace frst::literals;

namespace
{
using frst::testing::Dummy_Callable;

bool deep_eq(const Value_Ptr& lhs, const Value_Ptr& rhs)
{
    return Value::deep_equal(lhs, rhs)->get<Bool>().value();
}
} // namespace

TEST_CASE("Value clone")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).

    SECTION("Null clones to the singleton")
    {
        auto null_value = Value::null();
        auto clone = null_value->clone();
        CHECK(clone == null_value);
    }

    SECTION("Primitive values are cloned by value")
    {
        auto i1 = Value::create(42_f);
        auto i2 = i1->clone();
        CHECK(deep_eq(i1, i2));
        CHECK(i1 != i2);

        auto f1 = Value::create(3.25);
        auto f2 = f1->clone();
        CHECK(deep_eq(f1, f2));
        CHECK(f1 != f2);

        auto b1 = Value::create(true);
        auto b2 = b1->clone();
        CHECK(deep_eq(b1, b2));
        CHECK(b1 != b2);

        auto s1 = Value::create("hello"s);
        auto s2 = s1->clone();
        CHECK(deep_eq(s1, s2));
        CHECK(s1 != s2);
    }

    SECTION("Function clones preserve callable identity")
    {
        auto fn_ptr = std::make_shared<Dummy_Callable>();
        auto fn1 = Value::create(Function{fn_ptr});
        auto fn2 = fn1->clone();

        CHECK(fn1 != fn2);
        CHECK(fn1->raw_get<Function>() == fn2->raw_get<Function>());
        CHECK(deep_eq(fn1, fn2));
    }

    SECTION("Arrays clone recursively without preserving aliasing")
    {
        auto elem = Value::create(1_f);
        auto arr = Value::create(Array{elem, elem});

        auto clone = arr->clone();
        const auto& orig_arr = arr->raw_get<Array>();
        const auto& cloned_arr = clone->raw_get<Array>();

        REQUIRE(orig_arr.size() == 2);
        REQUIRE(cloned_arr.size() == 2);

        CHECK(orig_arr[0] == orig_arr[1]);
        CHECK(cloned_arr[0] != cloned_arr[1]);
        CHECK(cloned_arr[0] != elem);
        CHECK(cloned_arr[1] != elem);
        CHECK(arr != clone);
        CHECK(deep_eq(arr, clone));
    }

    SECTION("Maps clone recursively without preserving aliasing")
    {
        auto key1 = Value::create("a"s);
        auto key2 = Value::create("b"s);
        auto shared_val = Value::create(7_f);
        auto map = Value::create(Map{{key1, shared_val}, {key2, shared_val}});

        auto clone = map->clone();
        const auto& cloned_map = clone->raw_get<Map>();
        REQUIRE(cloned_map.size() == 2);

        std::vector<Value_Ptr> cloned_keys;
        std::vector<Value_Ptr> cloned_values;
        for (const auto& [k, v] : cloned_map)
        {
            cloned_keys.push_back(k);
            cloned_values.push_back(v);
        }

        REQUIRE(cloned_keys.size() == 2);
        REQUIRE(cloned_values.size() == 2);

        CHECK(cloned_keys[0] != key1);
        CHECK(cloned_keys[0] != key2);
        CHECK(cloned_keys[1] != key1);
        CHECK(cloned_keys[1] != key2);

        CHECK(cloned_values[0] != shared_val);
        CHECK(cloned_values[1] != shared_val);
        CHECK(cloned_values[0] != cloned_values[1]);

        CHECK(map != clone);
        CHECK(deep_eq(map, clone));
    }

    SECTION("Empty structures clone to new containers")
    {
        auto empty_arr = Value::create(Array{});
        auto empty_arr_clone = empty_arr->clone();
        CHECK(deep_eq(empty_arr, empty_arr_clone));
        CHECK(empty_arr != empty_arr_clone);

        auto empty_map = Value::create(Map{});
        auto empty_map_clone = empty_map->clone();
        CHECK(deep_eq(empty_map, empty_map_clone));
        CHECK(empty_map != empty_map_clone);
    }

    SECTION("Structured map keys are cloned")
    {
        auto key = Value::create(Array{Value::create(1_f)});
        auto value = Value::create(2_f);
        auto map = Value::create(Map{{key, value}});

        auto clone = map->clone();
        const auto& cloned_map = clone->raw_get<Map>();
        REQUIRE(cloned_map.size() == 1);

        const auto& [cloned_key, cloned_value] = *cloned_map.begin();
        CHECK(cloned_key != key);
        CHECK(cloned_value != value);
        CHECK(deep_eq(cloned_key, key));
        CHECK(deep_eq(cloned_value, value));
        CHECK(deep_eq(map, clone));
    }

    SECTION("Functions and nulls inside structures")
    {
        auto fn_ptr = std::make_shared<Dummy_Callable>();
        auto fn_value = Value::create(Function{fn_ptr});
        auto null_value = Value::null();

        auto arr = Value::create(Array{fn_value, null_value});
        auto clone = arr->clone();

        const auto& cloned_arr = clone->raw_get<Array>();
        REQUIRE(cloned_arr.size() == 2);

        auto cloned_fn = cloned_arr[0];
        auto cloned_null = cloned_arr[1];

        CHECK(cloned_fn != fn_value);
        CHECK(cloned_fn->raw_get<Function>() == fn_value->raw_get<Function>());

        CHECK(cloned_null == null_value);
        CHECK(cloned_null == Value::null());
        CHECK(deep_eq(arr, clone));
    }

    SECTION("Aliasing between map key and value is broken")
    {
        auto shared = Value::create("key"s);
        auto map = Value::create(Map{{shared, shared}});
        auto clone = map->clone();

        const auto& cloned_map = clone->raw_get<Map>();
        REQUIRE(cloned_map.size() == 1);

        const auto& [cloned_key, cloned_value] = *cloned_map.begin();
        CHECK(cloned_key != shared);
        CHECK(cloned_value != shared);
        CHECK(cloned_key != cloned_value);
        CHECK(deep_eq(map, clone));
    }

    SECTION("Aliasing in nested structures is broken")
    {
        auto inner = Value::create(Array{Value::create(1_f)});
        auto outer = Value::create(Array{inner, inner});
        auto clone = outer->clone();

        const auto& cloned_outer = clone->raw_get<Array>();
        REQUIRE(cloned_outer.size() == 2);

        auto first = cloned_outer[0];
        auto second = cloned_outer[1];
        CHECK(first != second);
        CHECK(first != inner);
        CHECK(second != inner);
        CHECK(deep_eq(first, inner));
        CHECK(deep_eq(second, inner));
        CHECK(deep_eq(outer, clone));
    }

    SECTION("Nested structures clone deeply")
    {
        auto inner = Value::create(Array{Value::create(1_f)});
        auto outer = Value::create(Array{inner});
        auto clone = outer->clone();

        const auto& cloned_outer = clone->raw_get<Array>();
        REQUIRE(cloned_outer.size() == 1);

        auto cloned_inner = cloned_outer[0];
        CHECK(cloned_inner != inner);

        const auto& inner_arr = inner->raw_get<Array>();
        const auto& cloned_inner_arr = cloned_inner->raw_get<Array>();
        REQUIRE(inner_arr.size() == 1);
        REQUIRE(cloned_inner_arr.size() == 1);
        CHECK(cloned_inner_arr[0] != inner_arr[0]);

        CHECK(deep_eq(outer, clone));
    }
}
