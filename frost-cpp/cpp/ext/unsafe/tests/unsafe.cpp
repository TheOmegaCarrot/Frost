#include <catch2/catch_all.hpp>

#include <frost/extensions-common.hpp>
#include <frost/value.hpp>

using namespace frst;
using namespace std::literals;

namespace frst
{
void register_module_unsafe(Stdlib_Registry_Builder&);
}

namespace
{

Function lookup(const std::string& name)
{
    static auto ext = [] {
        Stdlib_Registry_Builder builder;
        register_module_unsafe(builder);
        auto registry = std::move(builder).build();
        return registry.lookup_module("ext.unsafe").value()->raw_get<Map>();
    }();
    auto key = Value::create(String{name});
    return ext.at(key)->raw_get<Function>();
}

Function lookup_fn(const Map& map, const std::string& name)
{
    auto key = Value::create(String{name});
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
        "dummy")});
}

} // namespace

TEST_CASE("unsafe.same")
{
    auto same = lookup("same");

    SECTION("same value is same")
    {
        auto a = Value::create(Int{5});
        auto result = same->call({a, a});
        CHECK(result->get<Bool>().value() == true);
    }

    SECTION("equal but distinct values are not same")
    {
        auto a = Value::create(Int{5});
        auto b = Value::create(Int{5});
        auto result = same->call({a, b});
        CHECK(result->get<Bool>().value() == false);
    }

    SECTION("alias is same")
    {
        auto a = Value::create(String{"hello"});
        auto c = a;
        auto result = same->call({a, c});
        CHECK(result->get<Bool>().value() == true);
    }

    SECTION("different types are not same")
    {
        auto a = Value::create(Int{1});
        auto b = Value::create(Float{1.0});
        auto result = same->call({a, b});
        CHECK(result->get<Bool>().value() == false);
    }

    SECTION("null singletons are same")
    {
        auto a = Value::null();
        auto b = Value::null();
        auto result = same->call({a, b});
        CHECK(result->get<Bool>().value() == true);
    }

    SECTION("bool singletons are same")
    {
        auto a = Value::create(Bool{true});
        auto b = Value::create(Bool{true});
        auto result = same->call({a, b});
        CHECK(result->get<Bool>().value() == true);
    }

    SECTION("true and false are not same")
    {
        auto a = Value::create(Bool{true});
        auto b = Value::create(Bool{false});
        auto result = same->call({a, b});
        CHECK(result->get<Bool>().value() == false);
    }
}

TEST_CASE("unsafe.identity")
{
    auto identity = lookup("identity");

    SECTION("returns an Int")
    {
        auto a = Value::create(Int{42});
        auto result = identity->call({a});
        CHECK(result->get<Int>().has_value());
    }

    SECTION("same value gives same address")
    {
        auto a = Value::create(Int{42});
        auto r1 = identity->call({a});
        auto r2 = identity->call({a});
        CHECK(r1->get<Int>().value() == r2->get<Int>().value());
    }

    SECTION("alias gives same address")
    {
        auto a = Value::create(String{"hello"});
        auto c = a;
        auto r1 = identity->call({a});
        auto r2 = identity->call({c});
        CHECK(r1->get<Int>().value() == r2->get<Int>().value());
    }

    SECTION("distinct values give different addresses")
    {
        auto a = Value::create(Int{5});
        auto b = Value::create(Int{5});
        auto r1 = identity->call({a});
        auto r2 = identity->call({b});
        CHECK(r1->get<Int>().value() != r2->get<Int>().value());
    }

    SECTION("consistent with same")
    {
        auto same = lookup("same");
        auto a = Value::create(Int{5});
        auto b = Value::create(Int{5});

        bool same_says = same->call({a, b})->get<Bool>().value();
        bool ids_equal = identity->call({a})->get<Int>().value()
                         == identity->call({b})->get<Int>().value();
        CHECK(same_says == ids_equal);
    }
}

TEST_CASE("unsafe.mutable_cell")
{
    auto cell_fn = lookup("mutable_cell");

    SECTION("zero arguments creates a null cell")
    {
        auto cell = cell_fn->call({});
        REQUIRE(cell->is<Map>());
        auto get = lookup_fn(cell->raw_get<Map>(), "get");
        CHECK(get->call({}) == Value::null());
    }

    SECTION("get returns the stored value")
    {
        auto val = Value::create(Int{42});
        auto cell = cell_fn->call({val});
        auto get = lookup_fn(cell->raw_get<Map>(), "get");
        CHECK(get->call({}) == val);
        CHECK(get->call({}) == val);
    }

    SECTION("exchange returns old value and updates")
    {
        auto first = Value::create(Int{1});
        auto second = Value::create(Int{2});
        auto cell = cell_fn->call({first});
        const auto& m = cell->raw_get<Map>();
        auto get = lookup_fn(m, "get");
        auto exchange = lookup_fn(m, "exchange");

        auto old = exchange->call({second});
        CHECK(old == first);
        CHECK(get->call({}) == second);
    }

    SECTION("accepts function values")
    {
        auto fn = dummy_function();
        auto cell = cell_fn->call({fn});
        auto get = lookup_fn(cell->raw_get<Map>(), "get");
        CHECK(get->call({}) == fn);
    }

    SECTION("accepts function via exchange")
    {
        auto cell = cell_fn->call({Value::null()});
        const auto& m = cell->raw_get<Map>();
        auto get = lookup_fn(m, "get");
        auto exchange = lookup_fn(m, "exchange");

        auto fn = dummy_function();
        auto old = exchange->call({fn});
        CHECK(old == Value::null());
        CHECK(get->call({}) == fn);
    }

    SECTION("accepts arrays containing functions")
    {
        auto fn = dummy_function();
        auto arr = Value::create(Array{Value::create(Int{1}), fn});
        auto cell = cell_fn->call({arr});
        auto get = lookup_fn(cell->raw_get<Map>(), "get");
        CHECK(get->call({}) == arr);
    }

    SECTION("accepts maps containing functions")
    {
        auto fn = dummy_function();
        auto map = Value::create(
            Map{{Value::create(String{"cb"}), fn},
                {Value::create(String{"x"}), Value::create(Int{1})}});
        auto cell = cell_fn->call({map});
        auto get = lookup_fn(cell->raw_get<Map>(), "get");
        CHECK(get->call({}) == map);
    }

    SECTION("accepts another mutable cell as value")
    {
        auto inner = cell_fn->call({Value::create(Int{1})});
        auto outer = cell_fn->call({inner});
        auto get = lookup_fn(outer->raw_get<Map>(), "get");
        CHECK(get->call({}) == inner);
    }

    SECTION("different cells are isolated")
    {
        auto a0 = Value::create(Int{1});
        auto b0 = Value::create(Int{2});
        auto a1 = Value::create(Int{3});

        auto cell_a = cell_fn->call({a0});
        auto cell_b = cell_fn->call({b0});

        auto get_a = lookup_fn(cell_a->raw_get<Map>(), "get");
        auto ex_a = lookup_fn(cell_a->raw_get<Map>(), "exchange");
        auto get_b = lookup_fn(cell_b->raw_get<Map>(), "get");

        ex_a->call({a1});
        CHECK(get_a->call({}) == a1);
        CHECK(get_b->call({}) == b0);
    }

    SECTION("preserves pointer identity")
    {
        auto same = lookup("same");
        auto val = Value::create(String{"hello"});
        auto cell = cell_fn->call({val});
        auto get = lookup_fn(cell->raw_get<Map>(), "get");

        auto got = get->call({});
        CHECK(same->call({got, val})->get<Bool>().value() == true);
    }
}

TEST_CASE("unsafe.weaken")
{
    auto weaken = lookup("weaken");

    SECTION("get returns the value while it is alive")
    {
        auto val = Value::create(String{"hello"});
        auto weak = weaken->call({val});
        auto get = lookup_fn(weak->raw_get<Map>(), "get");

        auto got = get->call({});
        CHECK(got == val);
    }

    SECTION("get returns null after value is freed")
    {
        auto weak_map = [&] {
            auto val = Value::create(String{"ephemeral"});
            return weaken->call({val});
            // val is dropped here — last strong ref gone
        }();

        auto get = lookup_fn(weak_map->raw_get<Map>(), "get");
        CHECK(get->call({}) == Value::null());
    }

    SECTION("get transitions from alive to null")
    {
        auto val = Value::create(Array{Value::create(Int{1})});
        auto weak = weaken->call({val});
        auto get = lookup_fn(weak->raw_get<Map>(), "get");

        // alive while val exists
        CHECK(get->call({}) == val);

        // drop the strong ref
        val = Value::null();

        // now dead
        CHECK(get->call({}) == Value::null());
    }

    SECTION("multiple weak refs to same value")
    {
        auto val = Value::create(Int{42});
        auto weak1 = weaken->call({val});
        auto weak2 = weaken->call({val});
        auto get1 = lookup_fn(weak1->raw_get<Map>(), "get");
        auto get2 = lookup_fn(weak2->raw_get<Map>(), "get");

        CHECK(get1->call({}) == val);
        CHECK(get2->call({}) == val);

        val = Value::null();

        CHECK(get1->call({}) == Value::null());
        CHECK(get2->call({}) == Value::null());
    }

    SECTION("weak ref to null singleton never expires")
    {
        auto weak = weaken->call({Value::null()});
        auto get = lookup_fn(weak->raw_get<Map>(), "get");
        CHECK(get->call({}) == Value::null());
        // Can't distinguish "expired" from "holds null" — both return null.
        // This just verifies it doesn't crash.
    }

    SECTION("weak ref to bool singleton never expires")
    {
        auto val = Value::create(Bool{true});
        auto weak = weaken->call({val});
        auto get = lookup_fn(weak->raw_get<Map>(), "get");

        val = Value::null();

        // Bool is a singleton — other strong refs exist internally
        CHECK(get->call({}) != Value::null());
    }

    SECTION("works with mutable_cell exchange pattern")
    {
        auto cell_fn = lookup("mutable_cell");
        auto val = Value::create(
            Map{{Value::create(String{"x"}), Value::create(Int{1})}});

        auto cell = cell_fn->call({val});
        const auto& m = cell->raw_get<Map>();
        auto cell_get = lookup_fn(m, "get");
        auto exchange = lookup_fn(m, "exchange");

        auto weak = weaken->call({cell_get->call({})});
        auto weak_get = lookup_fn(weak->raw_get<Map>(), "get");

        // drop our local strong ref, cell still holds one
        val = Value::null();
        CHECK(weak_get->call({}) != Value::null());

        // drop the cell's strong ref
        exchange->call({Value::null()});
        CHECK(weak_get->call({}) == Value::null());
    }
}
