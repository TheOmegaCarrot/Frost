#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

using namespace frst;
using namespace std::literals;
using namespace frst::literals;
using Catch::Matchers::ContainsSubstring;

namespace
{
Function lookup_fn(const Map& map, const Value_Ptr& key)
{
    auto it = map.find(key);
    REQUIRE(it != map.end());
    REQUIRE(it->second->is<Function>());
    return it->second->get<Function>().value();
}

Value_Ptr dummy_function()
{
    return Value::create(Function{std::make_shared<Builtin>(
        [](builtin_args_t) {
            return Value::null();
        },
        "dummy", Builtin::Arity{.min = 0, .max = 0})});
}
} // namespace

TEST_CASE("Builtin mutable_cell")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).

    Symbol_Table table;
    inject_builtins(table);

    auto cell_val = table.lookup("mutable_cell");
    REQUIRE(cell_val->is<Function>());
    auto cell_fn = cell_val->get<Function>().value();

    SECTION("Arity: too many arguments")
    {
        CHECK_THROWS_WITH(cell_fn->call({Value::null(), Value::null()}),
                          ContainsSubstring("too many arguments"));
    }

    SECTION("Zero arguments creates a null cell")
    {
        auto cell = cell_fn->call({});
        REQUIRE(cell->is<Map>());
        const auto& cell_map = cell->raw_get<Map>();
        CHECK(cell_map.size() == 2);

        auto get_fn = lookup_fn(cell_map, "get"_s);
        auto exchange_fn = lookup_fn(cell_map, "exchange"_s);

        auto got = get_fn->call({});
        CHECK(got == Value::null());

        CHECK_THROWS_WITH(exchange_fn->call({}),
                          ContainsSubstring("insufficient arguments"));
    }

    SECTION("Get returns the stored value")
    {
        auto first = Value::create(1_f);
        auto cell = cell_fn->call({first});
        const auto& cell_map = cell->raw_get<Map>();
        auto get_fn = lookup_fn(cell_map, "get"_s);

        auto got1 = get_fn->call({});
        auto got2 = get_fn->call({});

        CHECK(got1 == first);
        CHECK(got2 == first);
    }

    SECTION("Get preserves pointer identity for all allowed value kinds")
    {
        auto arr_elem1 = Value::create(7_f);
        auto arr_elem2 = Value::create("x"s);
        auto arr = Value::create(Array{arr_elem1, arr_elem2});

        auto map_key = Value::create("k"s);
        auto map_val = Value::create(11_f);
        auto map = Value::create(Map{{map_key, map_val}});

        std::vector<Value_Ptr> values{
            Value::null(),
            Value::create(true),
            Value::create(1_f),
            Value::create(2.5),
            Value::create("abc"s),
            arr,
            map,
        };

        for (const auto& value : values)
        {
            auto cell = cell_fn->call({value});
            const auto& cell_map = cell->raw_get<Map>();
            auto get_fn = lookup_fn(cell_map, "get"_s);

            auto got1 = get_fn->call({});
            auto got2 = get_fn->call({});
            CHECK(got1 == value);
            CHECK(got2 == value);
        }
    }

    SECTION("Exchange returns the previous value and updates the cell")
    {
        auto first = Value::create(1_f);
        auto second = Value::create(2_f);
        auto cell = cell_fn->call({first});
        const auto& cell_map = cell->raw_get<Map>();
        auto get_fn = lookup_fn(cell_map, "get"_s);
        auto exchange_fn = lookup_fn(cell_map, "exchange"_s);

        auto previous = exchange_fn->call({second});
        CHECK(previous == first);

        auto after = get_fn->call({});
        CHECK(after == second);
    }

    SECTION("Exchange preserves pointer identity across allowed types")
    {
        auto arr_elem1 = Value::create(7_f);
        auto arr_elem2 = Value::create("x"s);
        auto arr = Value::create(Array{arr_elem1, arr_elem2});

        auto map_key = Value::create("k"s);
        auto map_val = Value::create(11_f);
        auto map = Value::create(Map{{map_key, map_val}});

        std::vector<Value_Ptr> values{
            Value::null(),
            Value::create(true),
            Value::create(1_f),
            Value::create(2.5),
            Value::create("abc"s),
            arr,
            map,
        };

        for (const auto& initial : values)
        {
            auto cell = cell_fn->call({initial});
            const auto& cell_map = cell->raw_get<Map>();
            auto get_fn = lookup_fn(cell_map, "get"_s);
            auto exchange_fn = lookup_fn(cell_map, "exchange"_s);

            auto current = initial;
            for (const auto& next : values)
            {
                auto previous = exchange_fn->call({next});
                CHECK(previous == current);
                CHECK(get_fn->call({}) == next);
                CHECK(get_fn->call({}) == next);
                current = next;
            }
        }
    }

    SECTION("Structured values are returned without deep cloning")
    {
        auto leaf1 = Value::create(3_f);
        auto leaf2 = Value::create("leaf"s);
        auto nested_arr = Value::create(Array{leaf1, leaf2});
        auto key_nested = Value::create("nested"s);
        auto key_other = Value::create("other"s);
        auto other = Value::create(9_f);
        auto nested_map =
            Value::create(Map{{key_nested, nested_arr}, {key_other, other}});

        auto cell = cell_fn->call({nested_map});
        const auto& cell_map = cell->raw_get<Map>();
        auto get_fn = lookup_fn(cell_map, "get"_s);

        auto got = get_fn->call({});
        REQUIRE(got == nested_map);
        REQUIRE(got->is<Map>());
        const auto& got_map = got->raw_get<Map>();

        bool saw_nested_key_identity = false;
        bool saw_other_key_identity = false;
        for (const auto& [k, _] : got_map)
        {
            saw_nested_key_identity =
                saw_nested_key_identity || (k == key_nested);
            saw_other_key_identity = saw_other_key_identity || (k == key_other);
        }
        CHECK(saw_nested_key_identity);
        CHECK(saw_other_key_identity);

        auto nested_it = got_map.find(key_nested);
        REQUIRE(nested_it != got_map.end());
        CHECK(nested_it->second == nested_arr);
        REQUIRE(nested_it->second->is<Array>());
        const auto& got_arr = nested_it->second->raw_get<Array>();
        REQUIRE(got_arr.size() == 2);
        CHECK(got_arr[0] == leaf1);
        CHECK(got_arr[1] == leaf2);
        auto other_it = got_map.find(key_other);
        REQUIRE(other_it != got_map.end());
        CHECK(other_it->second == other);
    }

    SECTION("Allows structured initial values with no functions")
    {
        auto nested = Value::create(Map{
            {Value::create("nums"s),
             Value::create(Array{Value::create(1_f), Value::create(2_f)})},
            {Value::create("inner"s),
             Value::create(Map{{Value::create("k"s), Value::create("v"s)}})},
        });

        auto cell = cell_fn->call({nested});
        const auto& cell_map = cell->raw_get<Map>();
        auto get_fn = lookup_fn(cell_map, "get"_s);
        auto got = get_fn->call({});
        CHECK(got == nested);
    }

    SECTION("Rejects storing another mutable_cell handle")
    {
        auto inner_cell = cell_fn->call({Value::create(1_f)});
        CHECK_THROWS_WITH(
            cell_fn->call({inner_cell}),
            ContainsSubstring("Function values may not be stored"));

        auto outer_cell = cell_fn->call({Value::create(2_f)});
        const auto& outer_map = outer_cell->raw_get<Map>();
        auto exchange_fn = lookup_fn(outer_map, "exchange"_s);
        CHECK_THROWS_WITH(
            exchange_fn->call({inner_cell}),
            ContainsSubstring("Function values may not be stored"));
    }

    SECTION("Rejects initial values containing a function recursively")
    {
        auto fn = dummy_function();

        auto array_with_fn = Value::create(Array{Value::create(1_f), fn});
        auto map_with_fn =
            Value::create(Map{{Value::create("k"s), Value::create(1_f)},
                              {Value::create("cb"s), fn}});
        auto deeply_nested_with_fn = Value::create(Map{
            {Value::create("a"s),
             Value::create(Array{Value::create(Map{
                 {Value::create("ok"s), Value::create(true)},
                 {Value::create("bad"s), fn},
             })})},
        });

        CHECK_THROWS_WITH(
            cell_fn->call({fn}),
            ContainsSubstring("Function values may not be stored"));
        CHECK_THROWS_WITH(
            cell_fn->call({array_with_fn}),
            ContainsSubstring("Function values may not be stored"));
        CHECK_THROWS_WITH(
            cell_fn->call({map_with_fn}),
            ContainsSubstring("Function values may not be stored"));
        CHECK_THROWS_WITH(
            cell_fn->call({deeply_nested_with_fn}),
            ContainsSubstring("Function values may not be stored"));
    }

    SECTION("Deeply nested structures are checked recursively")
    {
        Value_Ptr valid = Value::create(0_f);
        for (int i = 0; i < 128; ++i)
        {
            if ((i % 2) == 0)
                valid = Value::create(Array{valid});
            else
                valid = Value::create(Map{{Value::create(Int{i}), valid}});
        }

        auto ok_cell = cell_fn->call({valid});
        const auto& ok_map = ok_cell->raw_get<Map>();
        auto ok_get_fn = lookup_fn(ok_map, "get"_s);
        CHECK(ok_get_fn->call({}) == valid);

        Value_Ptr invalid = dummy_function();
        for (int i = 0; i < 128; ++i)
        {
            if ((i % 2) == 0)
                invalid = Value::create(Array{invalid});
            else
                invalid = Value::create(Map{{Value::create(Int{i}), invalid}});
        }

        CHECK_THROWS_WITH(
            cell_fn->call({invalid}),
            ContainsSubstring("Function values may not be stored"));
    }

    SECTION("Function-valued map keys are rejected recursively")
    {
        auto bad_key_map = Value::create(
            Value::trusted, Map{{dummy_function(), Value::create("payload"s)}});

        CHECK_THROWS_WITH(
            cell_fn->call({bad_key_map}),
            ContainsSubstring("Function values may not be stored"));
    }

    SECTION("Exchange arity errors")
    {
        auto cell = cell_fn->call({Value::create(1_f)});
        const auto& cell_map = cell->raw_get<Map>();
        auto exchange_fn = lookup_fn(cell_map, "exchange"_s);

        CHECK_THROWS_WITH(exchange_fn->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(exchange_fn->call({Value::null(), Value::null()}),
                          ContainsSubstring("too many arguments"));
    }

    SECTION("Get arity errors")
    {
        auto cell = cell_fn->call({Value::create(1_f)});
        const auto& cell_map = cell->raw_get<Map>();
        auto get_fn = lookup_fn(cell_map, "get"_s);

        CHECK_THROWS_WITH(get_fn->call({Value::null()}),
                          ContainsSubstring("too many arguments"));
    }

    SECTION("Allows structured exchange values with no functions")
    {
        auto cell = cell_fn->call({Value::create(1_f)});
        const auto& cell_map = cell->raw_get<Map>();
        auto get_fn = lookup_fn(cell_map, "get"_s);
        auto exchange_fn = lookup_fn(cell_map, "exchange"_s);

        auto nested = Value::create(Map{
            {Value::create("xs"s),
             Value::create(Array{Value::create(3_f), Value::create(4_f)})},
        });

        auto previous = exchange_fn->call({nested});
        CHECK(previous->get<Int>().value() == 1_f);
        CHECK(get_fn->call({}) == nested);
    }

    SECTION("Rejects exchange values containing a function recursively")
    {
        auto cell = cell_fn->call({Value::create(1_f)});
        const auto& cell_map = cell->raw_get<Map>();
        auto exchange_fn = lookup_fn(cell_map, "exchange"_s);

        auto fn = dummy_function();
        auto arr_with_fn = Value::create(Array{Value::create(1_f), fn});
        auto map_with_fn =
            Value::create(Map{{Value::create("k"s), Value::create(1_f)},
                              {Value::create("fn"s), fn}});
        auto nested_with_fn = Value::create(Map{
            {Value::create("outer"s), Value::create(Array{Value::create(Map{
                                          {Value::create("inner"s), fn},
                                      })})},
        });

        CHECK_THROWS_WITH(
            exchange_fn->call({fn}),
            ContainsSubstring("Function values may not be stored"));
        CHECK_THROWS_WITH(
            exchange_fn->call({arr_with_fn}),
            ContainsSubstring("Function values may not be stored"));
        CHECK_THROWS_WITH(
            exchange_fn->call({map_with_fn}),
            ContainsSubstring("Function values may not be stored"));
        CHECK_THROWS_WITH(
            exchange_fn->call({nested_with_fn}),
            ContainsSubstring("Function values may not be stored"));
    }

    SECTION("Failed exchange leaves the stored value unchanged")
    {
        auto key = Value::create("a"s);
        auto stored_leaf = Value::create(99_f);
        auto initial = Value::create(Map{{key, stored_leaf}});

        auto cell = cell_fn->call({initial});
        const auto& cell_map = cell->raw_get<Map>();
        auto get_fn = lookup_fn(cell_map, "get"_s);
        auto exchange_fn = lookup_fn(cell_map, "exchange"_s);

        auto invalid =
            Value::create(Array{Value::create("ok"s), dummy_function()});
        CHECK_THROWS_WITH(
            exchange_fn->call({invalid}),
            ContainsSubstring("Function values may not be stored"));

        auto got1 = get_fn->call({});
        auto got2 = get_fn->call({});
        CHECK(got1 == initial);
        CHECK(got2 == initial);

        REQUIRE(got1->is<Map>());
        const auto& got_map = got1->raw_get<Map>();
        auto it = got_map.find(key);
        REQUIRE(it != got_map.end());
        CHECK(it->second == stored_leaf);
    }

    SECTION("Different mutable cells remain isolated")
    {
        auto a0 = Value::create(1_f);
        auto b0 = Value::create(2_f);
        auto a1 = Value::create(3_f);
        auto b1 = Value::create(Map{
            {Value::create("k"s), Value::create("v"s)},
        });

        auto cell_a = cell_fn->call({a0});
        auto cell_b = cell_fn->call({b0});

        const auto& map_a = cell_a->raw_get<Map>();
        const auto& map_b = cell_b->raw_get<Map>();
        auto get_a = lookup_fn(map_a, "get"_s);
        auto ex_a = lookup_fn(map_a, "exchange"_s);
        auto get_b = lookup_fn(map_b, "get"_s);
        auto ex_b = lookup_fn(map_b, "exchange"_s);

        CHECK(get_a->call({}) == a0);
        CHECK(get_b->call({}) == b0);

        CHECK(ex_a->call({a1}) == a0);
        CHECK(get_a->call({}) == a1);
        CHECK(get_b->call({}) == b0);

        CHECK(ex_b->call({b1}) == b0);
        CHECK(get_b->call({}) == b1);
        CHECK(get_a->call({}) == a1);
    }
}
