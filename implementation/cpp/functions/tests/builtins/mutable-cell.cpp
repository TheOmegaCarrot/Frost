#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/testing/dummy-callable.hpp>
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
using frst::testing::Dummy_Callable;

bool deep_eq(const Value_Ptr& lhs, const Value_Ptr& rhs)
{
    return Value::deep_equal(lhs, rhs)->get<Bool>().value();
}

Function lookup_fn(const Map& map, const Value_Ptr& key)
{
    auto it = map.find(key);
    REQUIRE(it != map.end());
    REQUIRE(it->second->is<Function>());
    return it->second->get<Function>().value();
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

    SECTION("Get returns a clone of the stored value")
    {
        auto arr = Value::create(Array{Value::create(1_f)});
        auto cell = cell_fn->call({arr});
        const auto& cell_map = cell->raw_get<Map>();
        auto get_fn = lookup_fn(cell_map, "get"_s);

        auto got1 = get_fn->call({});
        auto got2 = get_fn->call({});

        CHECK(got1 != arr);
        CHECK(got2 != arr);
        CHECK(got1 != got2);
        CHECK(deep_eq(arr, got1));

        const auto& orig_arr = arr->raw_get<Array>();
        const auto& cloned_arr = got1->raw_get<Array>();
        REQUIRE(orig_arr.size() == 1);
        REQUIRE(cloned_arr.size() == 1);
        CHECK(cloned_arr[0] != orig_arr[0]);
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
        CHECK(after != second);
        CHECK(deep_eq(after, second));
    }

    SECTION("Exchange returns raw structured values and stores the new pointer")
    {
        auto first = Value::create(Array{Value::create(1_f)});
        auto second = Value::create(Array{Value::create(2_f)});
        auto cell = cell_fn->call({first});
        const auto& cell_map = cell->raw_get<Map>();
        auto get_fn = lookup_fn(cell_map, "get"_s);
        auto exchange_fn = lookup_fn(cell_map, "exchange"_s);

        auto prev = exchange_fn->call({second});
        CHECK(prev == first);

        auto prev2 = exchange_fn->call({first});
        CHECK(prev2 == second);

        auto after = get_fn->call({});
        CHECK(after != first);
        CHECK(deep_eq(after, first));
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

    SECTION("Function values preserve callable identity on get")
    {
        auto fn_ptr = std::make_shared<Dummy_Callable>();
        auto fn_val = Value::create(Function{fn_ptr});
        auto cell = cell_fn->call({fn_val});
        const auto& cell_map = cell->raw_get<Map>();
        auto get_fn = lookup_fn(cell_map, "get"_s);

        auto got = get_fn->call({});
        CHECK(got != fn_val);
        CHECK(got->raw_get<Function>() == fn_val->raw_get<Function>());
    }

    SECTION("Get clones maps deeply")
    {
        auto key = Value::create("k"s);
        auto val = Value::create(Array{Value::create(1_f)});
        auto map = Value::create(Map{{key, val}});
        auto cell = cell_fn->call({map});
        const auto& cell_map = cell->raw_get<Map>();
        auto get_fn = lookup_fn(cell_map, "get"_s);

        auto got = get_fn->call({});
        CHECK(got != map);
        CHECK(deep_eq(got, map));

        const auto& got_map = got->raw_get<Map>();
        REQUIRE(got_map.size() == 1);
        const auto& [got_key, got_val] = *got_map.begin();

        CHECK(got_key != key);
        CHECK(got_val != val);
        CHECK(deep_eq(got_key, key));
        CHECK(deep_eq(got_val, val));
    }
}
