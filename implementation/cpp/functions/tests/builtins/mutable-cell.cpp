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

    SECTION("Rejects non-primitive initial values")
    {
        auto arr = Value::create(Array{Value::create(1_f)});
        auto map =
            Value::create(Map{{Value::create("k"s), Value::create(1_f)}});
        auto fn = Value::create(Function{std::make_shared<Builtin>(
            [](builtin_args_t) { return Value::null(); }, "dummy",
            Builtin::Arity{.min = 0, .max = 0})});

        CHECK_THROWS_WITH(cell_fn->call({arr}),
                          ContainsSubstring("Non-primitive values"));
        CHECK_THROWS_WITH(cell_fn->call({map}),
                          ContainsSubstring("Non-primitive values"));
        CHECK_THROWS_WITH(cell_fn->call({fn}),
                          ContainsSubstring("Non-primitive values"));
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

    SECTION("Rejects non-primitive exchange values")
    {
        auto cell = cell_fn->call({Value::create(1_f)});
        const auto& cell_map = cell->raw_get<Map>();
        auto exchange_fn = lookup_fn(cell_map, "exchange"_s);

        auto arr = Value::create(Array{Value::create(1_f)});
        auto map =
            Value::create(Map{{Value::create("k"s), Value::create(1_f)}});
        auto fn = Value::create(Function{std::make_shared<Builtin>(
            [](builtin_args_t) { return Value::null(); }, "dummy",
            Builtin::Arity{.min = 0, .max = 0})});

        CHECK_THROWS_WITH(exchange_fn->call({arr}),
                          ContainsSubstring("Non-primitive values"));
        CHECK_THROWS_WITH(exchange_fn->call({map}),
                          ContainsSubstring("Non-primitive values"));
        CHECK_THROWS_WITH(exchange_fn->call({fn}),
                          ContainsSubstring("Non-primitive values"));
    }
}
