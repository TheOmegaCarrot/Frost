#include <catch2/catch_all.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <frost/builtin.hpp>

using namespace frst;

using namespace std::literals;
using namespace frst::literals;

using namespace Catch::Matchers;

TEST_CASE("Free Operators")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    Symbol_Table table;
    inject_builtins(table);

    auto get_fn = [&](const std::string& name) {
        auto val = table.lookup(name);
        REQUIRE(val->is<Function>());
        return val->get<Function>().value();
    };

    SECTION("Injected")
    {
        const std::vector<std::string> names{
            "plus",  "minus",          "times",       "divide",
            "equal", "not_equal",      "less_than",   "less_than_or_equal",
            "greater_than",            "greater_than_or_equal",
        };

        for (const auto& name : names)
        {
            auto val = table.lookup(name);
            REQUIRE(val->is<Function>());
        }
    }

    SECTION("Arity")
    {
        auto plus = get_fn("plus");
        CHECK_THROWS_WITH(plus->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(plus->call({}),
                          ContainsSubstring("Called with 0"));
        CHECK_THROWS_WITH(plus->call({}),
                          ContainsSubstring("requires at least 2"));

        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto c = Value::create(3_f);
        CHECK_THROWS_WITH(plus->call({a, b, c}),
                          ContainsSubstring("too many arguments"));
        CHECK_THROWS_WITH(plus->call({a, b, c}),
                          ContainsSubstring("Called with 3"));
        CHECK_THROWS_WITH(plus->call({a, b, c}),
                          ContainsSubstring("no more than 2"));
    }

    SECTION("Numeric arithmetic")
    {
        auto plus = get_fn("plus");
        auto minus = get_fn("minus");
        auto times = get_fn("times");
        auto divide = get_fn("divide");

        auto int1 = Value::create(42_f);
        auto int2 = Value::create(3_f);
        auto flt1 = Value::create(2.5);

        auto sum = plus->call({int1, int2});
        REQUIRE(sum->is<Int>());
        CHECK(sum->get<Int>().value() == 45_f);

        auto diff = minus->call({int1, int2});
        REQUIRE(diff->is<Int>());
        CHECK(diff->get<Int>().value() == 39_f);

        auto prod = times->call({int1, int2});
        REQUIRE(prod->is<Int>());
        CHECK(prod->get<Int>().value() == 126_f);

        auto quot = divide->call({int1, int2});
        REQUIRE(quot->is<Int>());
        CHECK(quot->get<Int>().value() == 14_f);

        auto mixed = plus->call({int2, flt1});
        REQUIRE(mixed->is<Float>());
        CHECK(mixed->get<Float>().value() == 3_f + 2.5);
    }

    SECTION("String and array concat")
    {
        auto plus = get_fn("plus");

        auto foo = Value::create("foo"s);
        auto bar = Value::create("bar"s);
        auto str = plus->call({foo, bar});
        REQUIRE(str->is<String>());
        CHECK(str->get<String>() == "foobar"s);

        auto arr1 =
            Value::create(Array{Value::create(1_f), Value::create(2_f)});
        auto arr2 =
            Value::create(Array{Value::create(true), Value::create("x"s)});
        auto arr = plus->call({arr1, arr2});
        REQUIRE(arr->is<Array>());
        auto bare = arr->get<Array>();
        REQUIRE(bare->size() == 4);
        CHECK(bare->at(0)->get<Int>() == 1_f);
        CHECK(bare->at(1)->get<Int>() == 2_f);
        CHECK(bare->at(2)->get<Bool>() == true);
        CHECK(bare->at(3)->get<String>() == "x"s);
    }

    SECTION("Equality and inequality")
    {
        auto eq = get_fn("equal");
        auto ne = get_fn("not_equal");

        auto a = Value::create(42_f);
        auto b = Value::create(42_f);
        auto c = Value::create(7_f);

        auto res_eq = eq->call({a, b});
        REQUIRE(res_eq->is<Bool>());
        CHECK(res_eq->get<Bool>().value() == true);

        auto res_ne = ne->call({a, c});
        REQUIRE(res_ne->is<Bool>());
        CHECK(res_ne->get<Bool>().value() == true);
    }

    SECTION("Comparisons")
    {
        auto lt = get_fn("less_than");
        auto lte = get_fn("less_than_or_equal");
        auto gt = get_fn("greater_than");
        auto gte = get_fn("greater_than_or_equal");

        auto small = Value::create(2_f);
        auto big = Value::create(10_f);

        CHECK(lt->call({small, big})->get<Bool>().value());
        CHECK_FALSE(lt->call({big, small})->get<Bool>().value());

        CHECK(lte->call({small, big})->get<Bool>().value());
        CHECK(lte->call({small, small})->get<Bool>().value());

        CHECK(gt->call({big, small})->get<Bool>().value());
        CHECK_FALSE(gt->call({small, big})->get<Bool>().value());

        CHECK(gte->call({big, small})->get<Bool>().value());
        CHECK(gte->call({big, big})->get<Bool>().value());
    }

    SECTION("Type errors")
    {
        auto plus = get_fn("plus");
        auto lt = get_fn("less_than");
        auto Null = Value::null();
        auto Int = Value::create(1_f);

        CHECK_THROWS_WITH(plus->call({Null, Int}),
                          "Cannot add incompatible types: Null + Int");
        CHECK_THROWS_WITH(lt->call({Null, Int}),
                          "Cannot compare incompatible types: Null < Int");
    }
}
