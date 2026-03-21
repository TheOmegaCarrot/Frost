#include <catch2/catch_all.hpp>

#include <frost/value.hpp>

#include <frost/extensions-common.hpp>

using namespace frst;
using namespace std::literals;

namespace frst
{
DECLARE_EXTENSION(unsafe);
}

namespace
{

Function lookup(const std::string& name)
{
    static Map ext = make_extension_unsafe();
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
        "dummy", Builtin::Arity{.min = 0, .max = 0})});
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
