#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_exception.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <vector>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

using namespace frst;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;
using Catch::Matchers::MessageMatches;

namespace
{
Function lookup(Symbol_Table& table, const std::string& name)
{
    auto val = table.lookup(name);
    REQUIRE(val->is<Function>());
    return val->get<Function>().value();
}

void require_array_eq(const Value_Ptr& value,
                      const std::vector<Value_Ptr>& expected)
{
    REQUIRE(value->is<Array>());
    const auto& arr = value->raw_get<Array>();
    REQUIRE(arr.size() == expected.size());
    for (std::size_t i = 0; i < expected.size(); ++i)
        CHECK(arr.at(i) == expected.at(i));
}
} // namespace

TEST_CASE("Builtin ranges")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).

    Symbol_Table table;
    inject_builtins(table);

    const std::vector<std::string> names{
        "stride", "take", "drop", "slide", "chunk",
    };
    const std::vector<std::string> unary_names{
        "reverse",
    };

    SECTION("Injected")
    {
        for (const auto& name : names)
        {
            auto val = table.lookup(name);
            REQUIRE(val->is<Function>());
        }
        for (const auto& name : unary_names)
        {
            auto val = table.lookup(name);
            REQUIRE(val->is<Function>());
        }
    }

    SECTION("Arity")
    {
        auto arr = Value::create(Array{});
        auto n = Value::create(1_f);
        auto extra = Value::create(2_f);

        for (const auto& name : names)
        {
            DYNAMIC_SECTION("Arity " << name)
            {
                auto fn = lookup(table, name);
                CHECK_THROWS_MATCHES(
                    fn->call({}), Frost_User_Error,
                    MessageMatches(ContainsSubstring("insufficient arguments")
                                   && ContainsSubstring("requires at least 2")
                                   && ContainsSubstring("Called with 0")));
                CHECK_THROWS_MATCHES(
                    fn->call({arr}), Frost_User_Error,
                    MessageMatches(ContainsSubstring("insufficient arguments")
                                   && ContainsSubstring("requires at least 2")
                                   && ContainsSubstring("Called with 1")));
                CHECK_THROWS_MATCHES(
                    fn->call({arr, n, extra}), Frost_User_Error,
                    MessageMatches(ContainsSubstring("too many arguments")
                                   && ContainsSubstring("no more than 2")
                                   && ContainsSubstring("Called with 3")));
            }
        }

        for (const auto& name : unary_names)
        {
            DYNAMIC_SECTION("Arity " << name)
            {
                auto fn = lookup(table, name);
                CHECK_THROWS_MATCHES(
                    fn->call({}), Frost_User_Error,
                    MessageMatches(ContainsSubstring("insufficient arguments")
                                   && ContainsSubstring("requires at least 1")
                                   && ContainsSubstring("Called with 0")));
                CHECK_THROWS_MATCHES(
                    fn->call({arr, n}), Frost_User_Error,
                    MessageMatches(ContainsSubstring("too many arguments")
                                   && ContainsSubstring("no more than 1")
                                   && ContainsSubstring("Called with 2")));
            }
        }
    }

    SECTION("Type errors")
    {
        auto bad = Value::create("nope"s);
        auto good_arr = Value::create(Array{Value::create(1_f)});
        auto good_int = Value::create(1_f);

        for (const auto& name : names)
        {
            DYNAMIC_SECTION("Type " << name)
            {
                auto fn = lookup(table, name);
                CHECK_THROWS_MATCHES(
                    fn->call({bad, good_int}), Frost_User_Error,
                    MessageMatches(ContainsSubstring("Function " + name)
                                   && ContainsSubstring("Array")
                                   && ContainsSubstring("got String")));
                CHECK_THROWS_MATCHES(
                    fn->call({good_arr, bad}), Frost_User_Error,
                    MessageMatches(ContainsSubstring("Function " + name)
                                   && ContainsSubstring("Int")
                                   && ContainsSubstring("got String")));
                CHECK_THROWS_MATCHES(
                    fn->call({good_arr, Value::create(2.5)}), Frost_User_Error,
                    MessageMatches(ContainsSubstring("Function " + name)
                                   && ContainsSubstring("Int")
                                   && ContainsSubstring("got Float")));
            }
        }

        for (const auto& name : unary_names)
        {
            DYNAMIC_SECTION("Type " << name)
            {
                auto fn = lookup(table, name);
                CHECK_THROWS_MATCHES(
                    fn->call({bad}), Frost_User_Error,
                    MessageMatches(ContainsSubstring("Function " + name)
                                   && ContainsSubstring("Array")
                                   && ContainsSubstring("got String")));
                CHECK_THROWS_MATCHES(
                    fn->call({Value::create(2.5)}), Frost_User_Error,
                    MessageMatches(ContainsSubstring("Function " + name)
                                   && ContainsSubstring("Array")
                                   && ContainsSubstring("got Float")));
            }
        }
    }

    SECTION("stride semantics")
    {
        auto fn = lookup(table, "stride");
        auto a = Value::create(0_f);
        auto b = Value::create(1_f);
        auto c = Value::create(2_f);
        auto d = Value::create(3_f);
        auto e = Value::create(4_f);
        auto arr = Value::create(Array{a, b, c, d, e});

        require_array_eq(fn->call({arr, Value::create(2_f)}), {a, c, e});
        require_array_eq(fn->call({arr, Value::create(3_f)}), {a, d});
        require_array_eq(fn->call({arr, Value::create(10_f)}), {a});
        require_array_eq(fn->call({arr, Value::create(1_f)}), {a, b, c, d, e});

        auto empty_arr = Value::create(Array{});
        require_array_eq(fn->call({empty_arr, Value::create(1_f)}), {});

        CHECK_THROWS_AS(fn->call({arr, Value::create(0_f)}), Frost_User_Error);
        CHECK_THROWS_AS(fn->call({arr, Value::create(-1_f)}), Frost_User_Error);
    }

    SECTION("take semantics")
    {
        auto fn = lookup(table, "take");
        auto a = Value::create(0_f);
        auto b = Value::create(1_f);
        auto c = Value::create(2_f);
        auto d = Value::create(3_f);
        auto e = Value::create(4_f);
        auto arr = Value::create(Array{a, b, c, d, e});

        require_array_eq(fn->call({arr, Value::create(0_f)}), {});
        require_array_eq(fn->call({arr, Value::create(2_f)}), {a, b});
        require_array_eq(fn->call({arr, Value::create(10_f)}), {a, b, c, d, e});
        require_array_eq(fn->call({arr, Value::create(5_f)}), {a, b, c, d, e});
        require_array_eq(fn->call({arr, Value::create(4_f)}), {a, b, c, d});

        auto empty_arr = Value::create(Array{});
        require_array_eq(fn->call({empty_arr, Value::create(2_f)}), {});

        CHECK_THROWS_AS(fn->call({arr, Value::create(-1_f)}), Frost_User_Error);
    }

    SECTION("drop semantics")
    {
        auto fn = lookup(table, "drop");
        auto a = Value::create(0_f);
        auto b = Value::create(1_f);
        auto c = Value::create(2_f);
        auto d = Value::create(3_f);
        auto e = Value::create(4_f);
        auto arr = Value::create(Array{a, b, c, d, e});

        require_array_eq(fn->call({arr, Value::create(0_f)}), {a, b, c, d, e});
        require_array_eq(fn->call({arr, Value::create(2_f)}), {c, d, e});
        require_array_eq(fn->call({arr, Value::create(10_f)}), {});
        require_array_eq(fn->call({arr, Value::create(5_f)}), {});
        require_array_eq(fn->call({arr, Value::create(4_f)}), {e});

        auto empty_arr = Value::create(Array{});
        require_array_eq(fn->call({empty_arr, Value::create(2_f)}), {});

        CHECK_THROWS_AS(fn->call({arr, Value::create(-1_f)}), Frost_User_Error);
    }

    SECTION("slide semantics")
    {
        auto fn = lookup(table, "slide");
        auto a = Value::create(0_f);
        auto b = Value::create(1_f);
        auto c = Value::create(2_f);
        auto d = Value::create(3_f);
        auto e = Value::create(4_f);
        auto arr = Value::create(Array{a, b, c, d, e});

        auto res = fn->call({arr, Value::create(2_f)});
        REQUIRE(res->is<Array>());
        const auto& outer = res->raw_get<Array>();
        REQUIRE(outer.size() == 4);
        require_array_eq(outer.at(0), {a, b});
        require_array_eq(outer.at(1), {b, c});
        require_array_eq(outer.at(2), {c, d});
        require_array_eq(outer.at(3), {d, e});

        auto full = fn->call({arr, Value::create(5_f)});
        REQUIRE(full->is<Array>());
        const auto& full_outer = full->raw_get<Array>();
        REQUIRE(full_outer.size() == 1);
        require_array_eq(full_outer.at(0), {a, b, c, d, e});

        auto empty = fn->call({arr, Value::create(6_f)});
        REQUIRE(empty->is<Array>());
        CHECK(empty->raw_get<Array>().empty());

        auto res_one = fn->call({arr, Value::create(1_f)});
        REQUIRE(res_one->is<Array>());
        const auto& one_outer = res_one->raw_get<Array>();
        REQUIRE(one_outer.size() == 5);
        require_array_eq(one_outer.at(0), {a});
        require_array_eq(one_outer.at(1), {b});
        require_array_eq(one_outer.at(2), {c});
        require_array_eq(one_outer.at(3), {d});
        require_array_eq(one_outer.at(4), {e});

        auto empty_arr = Value::create(Array{});
        auto empty_out = fn->call({empty_arr, Value::create(2_f)});
        REQUIRE(empty_out->is<Array>());
        CHECK(empty_out->raw_get<Array>().empty());

        auto empty_out_big = fn->call({empty_arr, Value::create(5_f)});
        REQUIRE(empty_out_big->is<Array>());
        CHECK(empty_out_big->raw_get<Array>().empty());

        CHECK_THROWS_AS(fn->call({arr, Value::create(0_f)}), Frost_User_Error);
        CHECK_THROWS_AS(fn->call({arr, Value::create(-1_f)}), Frost_User_Error);
    }

    SECTION("chunk semantics")
    {
        auto fn = lookup(table, "chunk");
        auto a = Value::create(0_f);
        auto b = Value::create(1_f);
        auto c = Value::create(2_f);
        auto d = Value::create(3_f);
        auto e = Value::create(4_f);
        auto arr = Value::create(Array{a, b, c, d, e});

        auto res = fn->call({arr, Value::create(2_f)});
        REQUIRE(res->is<Array>());
        const auto& outer = res->raw_get<Array>();
        REQUIRE(outer.size() == 3);
        require_array_eq(outer.at(0), {a, b});
        require_array_eq(outer.at(1), {c, d});
        require_array_eq(outer.at(2), {e});

        auto full = fn->call({arr, Value::create(5_f)});
        REQUIRE(full->is<Array>());
        const auto& full_outer = full->raw_get<Array>();
        REQUIRE(full_outer.size() == 1);
        require_array_eq(full_outer.at(0), {a, b, c, d, e});

        auto over = fn->call({arr, Value::create(6_f)});
        REQUIRE(over->is<Array>());
        const auto& over_outer = over->raw_get<Array>();
        REQUIRE(over_outer.size() == 1);
        require_array_eq(over_outer.at(0), {a, b, c, d, e});

        auto res_one = fn->call({arr, Value::create(1_f)});
        REQUIRE(res_one->is<Array>());
        const auto& one_outer = res_one->raw_get<Array>();
        REQUIRE(one_outer.size() == 5);
        require_array_eq(one_outer.at(0), {a});
        require_array_eq(one_outer.at(1), {b});
        require_array_eq(one_outer.at(2), {c});
        require_array_eq(one_outer.at(3), {d});
        require_array_eq(one_outer.at(4), {e});

        auto empty_arr = Value::create(Array{});
        auto empty_out = fn->call({empty_arr, Value::create(2_f)});
        REQUIRE(empty_out->is<Array>());
        CHECK(empty_out->raw_get<Array>().empty());

        auto empty_out_big = fn->call({empty_arr, Value::create(5_f)});
        REQUIRE(empty_out_big->is<Array>());
        CHECK(empty_out_big->raw_get<Array>().empty());

        CHECK_THROWS_AS(fn->call({arr, Value::create(0_f)}), Frost_User_Error);
        CHECK_THROWS_AS(fn->call({arr, Value::create(-1_f)}), Frost_User_Error);
    }

    SECTION("zip semantics")
    {
        auto fn = lookup(table, "zip");
        auto a = Value::create(0_f);
        auto b = Value::create(1_f);
        auto c = Value::create(2_f);
        auto d = Value::create(3_f);
        auto e = Value::create(4_f);
        auto f = Value::create(5_f);

        auto arr1 = Value::create(Array{a, b});
        auto arr2 = Value::create(Array{c, d, e});

        auto res = fn->call({arr1, arr2});
        REQUIRE(res->is<Array>());
        const auto& outer = res->raw_get<Array>();
        REQUIRE(outer.size() == 2);
        require_array_eq(outer.at(0), {a, c});
        require_array_eq(outer.at(1), {b, d});

        auto empty = Value::create(Array{});
        auto res_empty = fn->call({arr1, empty});
        REQUIRE(res_empty->is<Array>());
        CHECK(res_empty->raw_get<Array>().empty());

        auto res_more = fn->call({arr1, arr2, Value::create(Array{d, e})});
        REQUIRE(res_more->is<Array>());
        const auto& outer_more = res_more->raw_get<Array>();
        REQUIRE(outer_more.size() == 2);
        require_array_eq(outer_more.at(0), {a, c, d});
        require_array_eq(outer_more.at(1), {b, d, e});

        auto res_equal =
            fn->call({arr1, Value::create(Array{c, d}), Value::create(Array{e, f})});
        REQUIRE(res_equal->is<Array>());
        const auto& outer_equal = res_equal->raw_get<Array>();
        REQUIRE(outer_equal.size() == 2);
        require_array_eq(outer_equal.at(0), {a, c, e});
        require_array_eq(outer_equal.at(1), {b, d, f});

        auto single =
            fn->call({Value::create(Array{a}), Value::create(Array{b}),
                      Value::create(Array{c}), Value::create(Array{d})});
        REQUIRE(single->is<Array>());
        const auto& single_outer = single->raw_get<Array>();
        REQUIRE(single_outer.size() == 1);
        require_array_eq(single_outer.at(0), {a, b, c, d});

        auto all_empty =
            fn->call({Value::create(Array{}), Value::create(Array{})});
        REQUIRE(all_empty->is<Array>());
        CHECK(all_empty->raw_get<Array>().empty());

        auto mixed_empty =
            fn->call({Value::create(Array{}), Value::create(Array{a}),
                      Value::create(Array{b, c})});
        REQUIRE(mixed_empty->is<Array>());
        CHECK(mixed_empty->raw_get<Array>().empty());
    }

    SECTION("zip arity")
    {
        auto fn = lookup(table, "zip");
        auto arr = Value::create(Array{Value::create(1_f)});

        CHECK_THROWS_MATCHES(
            fn->call({}), Frost_User_Error,
            MessageMatches(ContainsSubstring("insufficient arguments")
                           && ContainsSubstring("requires at least 2")
                           && ContainsSubstring("Called with 0")));
        CHECK_THROWS_MATCHES(
            fn->call({arr}), Frost_User_Error,
            MessageMatches(ContainsSubstring("insufficient arguments")
                           && ContainsSubstring("requires at least 2")
                           && ContainsSubstring("Called with 1")));
    }

    SECTION("zip type errors")
    {
        auto fn = lookup(table, "zip");
        auto arr = Value::create(Array{Value::create(1_f)});
        auto bad = Value::create("nope"s);

        CHECK_THROWS_WITH(fn->call({bad, arr}),
                          ContainsSubstring("argument 0"));
        CHECK_THROWS_WITH(fn->call({arr, bad}),
                          ContainsSubstring("argument 1"));
        CHECK_THROWS_WITH(fn->call({arr, arr, bad}),
                          ContainsSubstring("argument 2"));
    }

    SECTION("reverse semantics")
    {
        auto fn = lookup(table, "reverse");
        auto a = Value::create(0_f);
        auto b = Value::create(1_f);
        auto c = Value::create(2_f);
        auto arr = Value::create(Array{a, b, c});

        require_array_eq(fn->call({arr}), {c, b, a});

        auto empty = Value::create(Array{});
        require_array_eq(fn->call({empty}), {});

        auto single = Value::create(Array{a});
        require_array_eq(fn->call({single}), {a});
    }
}
