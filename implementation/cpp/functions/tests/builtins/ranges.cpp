#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_exception.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>
#include <catch2/trompeloeil.hpp>

#include <vector>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/mock/mock-callable.hpp>

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

using namespace frst;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;
using Catch::Matchers::MessageMatches;
using trompeloeil::_;

namespace
{
template <typename Fn>
Function make_builtin(Fn fn, std::string name, Builtin::Arity arity)
{
    return std::make_shared<Builtin>(std::move(fn), std::move(name), arity);
}

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

Int require_int(const Value_Ptr& value)
{
    REQUIRE(value->is<Int>());
    return value->raw_get<Int>();
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
    const std::vector<std::string> variadic_names{
        "zip",
        "xprod",
    };
    const std::vector<std::string> pred_names{
        "take_while",
        "drop_while",
        "chunk_by",
        "group_by",
        "count_by",
        "scan",
    };
    const std::vector<std::string> maplike_names{
        "transform",
        "select",
    };
    const std::vector<std::string> quantifier_names{
        "any",
        "all",
        "none",
    };
    const std::vector<std::string> optional_pred_names{
        "sorted",
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
        for (const auto& name : variadic_names)
        {
            auto val = table.lookup(name);
            REQUIRE(val->is<Function>());
        }
        for (const auto& name : pred_names)
        {
            auto val = table.lookup(name);
            REQUIRE(val->is<Function>());
        }
        for (const auto& name : maplike_names)
        {
            auto val = table.lookup(name);
            REQUIRE(val->is<Function>());
        }
        for (const auto& name : quantifier_names)
        {
            auto val = table.lookup(name);
            REQUIRE(val->is<Function>());
        }
        for (const auto& name : optional_pred_names)
        {
            auto val = table.lookup(name);
            REQUIRE(val->is<Function>());
        }
        {
            auto val = table.lookup("fold");
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

        for (const auto& name : variadic_names)
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
            }
        }

        for (const auto& name : pred_names)
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

        for (const auto& name : maplike_names)
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

        for (const auto& name : quantifier_names)
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
                    fn->call({arr, n, extra}), Frost_User_Error,
                    MessageMatches(ContainsSubstring("too many arguments")
                                   && ContainsSubstring("no more than 2")
                                   && ContainsSubstring("Called with 3")));
            }
        }

        for (const auto& name : optional_pred_names)
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
                    fn->call({arr, n, extra}), Frost_User_Error,
                    MessageMatches(ContainsSubstring("too many arguments")
                                   && ContainsSubstring("no more than 2")
                                   && ContainsSubstring("Called with 3")));
            }
        }

        {
            auto fn = lookup(table, "fold");
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
                fn->call({arr, n, extra, extra}), Frost_User_Error,
                MessageMatches(ContainsSubstring("too many arguments")
                               && ContainsSubstring("no more than 3")
                               && ContainsSubstring("Called with 4")));
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

        for (const auto& name : variadic_names)
        {
            DYNAMIC_SECTION("Type " << name)
            {
                auto fn = lookup(table, name);
                CHECK_THROWS_MATCHES(
                    fn->call({bad, good_arr}), Frost_User_Error,
                    MessageMatches(ContainsSubstring("Function " + name)
                                   && ContainsSubstring("Array")
                                   && ContainsSubstring("argument 0")
                                   && ContainsSubstring("got String")));
                CHECK_THROWS_MATCHES(
                    fn->call({good_arr, bad}), Frost_User_Error,
                    MessageMatches(ContainsSubstring("Function " + name)
                                   && ContainsSubstring("Array")
                                   && ContainsSubstring("argument 1")
                                   && ContainsSubstring("got String")));
                CHECK_THROWS_MATCHES(
                    fn->call({good_arr, good_arr, bad}), Frost_User_Error,
                    MessageMatches(ContainsSubstring("Function " + name)
                                   && ContainsSubstring("Array")
                                   && ContainsSubstring("argument 2")
                                   && ContainsSubstring("got String")));
            }
        }

        for (const auto& name : pred_names)
        {
            DYNAMIC_SECTION("Type " << name)
            {
                auto fn = lookup(table, name);
                CHECK_THROWS_MATCHES(
                    fn->call({bad, good_arr}), Frost_User_Error,
                    MessageMatches(ContainsSubstring("Function " + name)
                                   && ContainsSubstring("Array")
                                   && ContainsSubstring("argument 1")
                                   && ContainsSubstring("got String")));
                CHECK_THROWS_MATCHES(
                    fn->call({good_arr, bad}), Frost_User_Error,
                    MessageMatches(ContainsSubstring("Function " + name)
                                   && ContainsSubstring("Function")
                                   && ContainsSubstring("argument 2")
                                   && ContainsSubstring("got String")));
            }
        }

        for (const auto& name : maplike_names)
        {
            DYNAMIC_SECTION("Type " << name)
            {
                auto fn = lookup(table, name);
                CHECK_THROWS_MATCHES(
                    fn->call({bad, good_arr}), Frost_User_Error,
                    MessageMatches(ContainsSubstring("Function " + name)
                                   && ContainsSubstring("Array or Map")
                                   && ContainsSubstring("argument 1")
                                   && ContainsSubstring("structure")
                                   && ContainsSubstring("got String")));
                CHECK_THROWS_MATCHES(
                    fn->call({good_arr, bad}), Frost_User_Error,
                    MessageMatches(ContainsSubstring("Function " + name)
                                   && ContainsSubstring("Function")
                                   && ContainsSubstring("argument 2")
                                   && ContainsSubstring("got String")));
            }
        }

        for (const auto& name : quantifier_names)
        {
            DYNAMIC_SECTION("Type " << name)
            {
                auto fn = lookup(table, name);
                CHECK_THROWS_MATCHES(
                    fn->call({bad}), Frost_User_Error,
                    MessageMatches(ContainsSubstring("Function " + name)
                                   && ContainsSubstring("Array")
                                   && ContainsSubstring("argument 1")
                                   && ContainsSubstring("got String")));
                CHECK_THROWS_MATCHES(
                    fn->call({good_arr, bad}), Frost_User_Error,
                    MessageMatches(ContainsSubstring("Function " + name)
                                   && ContainsSubstring("Function")
                                   && ContainsSubstring("argument 2")
                                   && ContainsSubstring("got String")));
            }
        }

        for (const auto& name : optional_pred_names)
        {
            DYNAMIC_SECTION("Type " << name)
            {
                auto fn = lookup(table, name);
                CHECK_THROWS_MATCHES(
                    fn->call({bad}), Frost_User_Error,
                    MessageMatches(ContainsSubstring("Function " + name)
                                   && ContainsSubstring("Array")
                                   && ContainsSubstring("argument 1")
                                   && ContainsSubstring("got String")));
                CHECK_THROWS_MATCHES(
                    fn->call({good_arr, bad}), Frost_User_Error,
                    MessageMatches(ContainsSubstring("Function " + name)
                                   && ContainsSubstring("Function")
                                   && ContainsSubstring("argument 2")
                                   && ContainsSubstring("got String")));
            }
        }

        {
            auto fn = lookup(table, "fold");
            CHECK_THROWS_MATCHES(
                fn->call({bad, good_arr}), Frost_User_Error,
                MessageMatches(ContainsSubstring("Function fold")
                               && ContainsSubstring("Array or Map")
                               && ContainsSubstring("argument 1")
                               && ContainsSubstring("structure")
                               && ContainsSubstring("got String")));
            CHECK_THROWS_MATCHES(
                fn->call({good_arr, bad}), Frost_User_Error,
                MessageMatches(ContainsSubstring("Function fold")
                               && ContainsSubstring("Function")
                               && ContainsSubstring("argument 2")
                               && ContainsSubstring("got String")));
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

        auto res_equal = fn->call(
            {arr1, Value::create(Array{c, d}), Value::create(Array{e, f})});
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

    SECTION("xprod semantics")
    {
        auto fn = lookup(table, "xprod");
        auto a = Value::create(0_f);
        auto b = Value::create(1_f);
        auto c = Value::create(2_f);
        auto d = Value::create(3_f);
        auto e = Value::create(4_f);
        auto f = Value::create(5_f);
        auto g = Value::create(6_f);
        auto h = Value::create(7_f);

        auto arr1 = Value::create(Array{a, b});
        auto arr2 = Value::create(Array{c, d});

        auto res = fn->call({arr1, arr2});
        REQUIRE(res->is<Array>());
        const auto& outer = res->raw_get<Array>();
        REQUIRE(outer.size() == 4);
        require_array_eq(outer.at(0), {a, c});
        require_array_eq(outer.at(1), {a, d});
        require_array_eq(outer.at(2), {b, c});
        require_array_eq(outer.at(3), {b, d});

        auto arr3 = Value::create(Array{e, f});
        auto res_three = fn->call({arr1, arr2, arr3});
        REQUIRE(res_three->is<Array>());
        const auto& outer_three = res_three->raw_get<Array>();
        REQUIRE(outer_three.size() == 8);
        require_array_eq(outer_three.at(0), {a, c, e});
        require_array_eq(outer_three.at(1), {a, c, f});
        require_array_eq(outer_three.at(2), {a, d, e});
        require_array_eq(outer_three.at(3), {a, d, f});
        require_array_eq(outer_three.at(4), {b, c, e});
        require_array_eq(outer_three.at(5), {b, c, f});
        require_array_eq(outer_three.at(6), {b, d, e});
        require_array_eq(outer_three.at(7), {b, d, f});

        auto arr4 = Value::create(Array{g, h});
        auto res_four = fn->call({arr1, arr2, arr3, arr4});
        REQUIRE(res_four->is<Array>());
        const auto& outer_four = res_four->raw_get<Array>();
        REQUIRE(outer_four.size() == 16);
        require_array_eq(outer_four.at(0), {a, c, e, g});
        require_array_eq(outer_four.at(1), {a, c, e, h});
        require_array_eq(outer_four.at(2), {a, c, f, g});
        require_array_eq(outer_four.at(3), {a, c, f, h});
        require_array_eq(outer_four.at(4), {a, d, e, g});
        require_array_eq(outer_four.at(5), {a, d, e, h});
        require_array_eq(outer_four.at(6), {a, d, f, g});
        require_array_eq(outer_four.at(7), {a, d, f, h});
        require_array_eq(outer_four.at(8), {b, c, e, g});
        require_array_eq(outer_four.at(9), {b, c, e, h});
        require_array_eq(outer_four.at(10), {b, c, f, g});
        require_array_eq(outer_four.at(11), {b, c, f, h});
        require_array_eq(outer_four.at(12), {b, d, e, g});
        require_array_eq(outer_four.at(13), {b, d, e, h});
        require_array_eq(outer_four.at(14), {b, d, f, g});
        require_array_eq(outer_four.at(15), {b, d, f, h});

        auto single = Value::create(Array{a});
        auto res_single_two = fn->call({single, arr2});
        REQUIRE(res_single_two->is<Array>());
        const auto& outer_single_two = res_single_two->raw_get<Array>();
        REQUIRE(outer_single_two.size() == 2);
        require_array_eq(outer_single_two.at(0), {a, c});
        require_array_eq(outer_single_two.at(1), {a, d});

        auto res_single_three = fn->call({single, arr2, arr3});
        REQUIRE(res_single_three->is<Array>());
        const auto& outer_single_three = res_single_three->raw_get<Array>();
        REQUIRE(outer_single_three.size() == 4);
        require_array_eq(outer_single_three.at(0), {a, c, e});
        require_array_eq(outer_single_three.at(1), {a, c, f});
        require_array_eq(outer_single_three.at(2), {a, d, e});
        require_array_eq(outer_single_three.at(3), {a, d, f});

        auto empty = Value::create(Array{});
        auto res_empty = fn->call({arr1, empty});
        REQUIRE(res_empty->is<Array>());
        CHECK(res_empty->raw_get<Array>().empty());
    }

    SECTION("take_while semantics")
    {
        auto fn = lookup(table, "take_while");
        auto a = Value::create(0_f);
        auto b = Value::create(1_f);
        auto c = Value::create(2_f);
        auto d = Value::create(3_f);
        auto arr = Value::create(Array{a, b, c, d});

        std::size_t call_count = 0;
        std::vector<Value_Ptr> seen;
        auto pred = Value::create(Function{std::make_shared<Builtin>(
            [&](builtin_args_t args) {
                ++call_count;
                seen.push_back(args.at(0));
                return Value::create(args.at(0)->raw_get<Int>() < 2_f);
            },
            "pred", Builtin::Arity{.min = 1, .max = 1})});

        auto res = fn->call({arr, pred});
        require_array_eq(res, {a, b});
        CHECK(call_count >= 3);
        REQUIRE(seen.size() >= 3);
        CHECK(std::ranges::find(seen, a) != seen.end());
        CHECK(std::ranges::find(seen, b) != seen.end());
        CHECK(std::ranges::find(seen, c) != seen.end());
        CHECK(std::ranges::find(seen, d) == seen.end());

        call_count = 0;
        seen.clear();
        auto pred_all = Value::create(Function{std::make_shared<Builtin>(
            [&](builtin_args_t args) {
                ++call_count;
                seen.push_back(args.at(0));
                return Value::create(true);
            },
            "pred_all", Builtin::Arity{.min = 1, .max = 1})});

        auto res_all = fn->call({arr, pred_all});
        require_array_eq(res_all, {a, b, c, d});
        CHECK(call_count >= 4);
        CHECK(std::ranges::find(seen, a) != seen.end());
        CHECK(std::ranges::find(seen, b) != seen.end());
        CHECK(std::ranges::find(seen, c) != seen.end());
        CHECK(std::ranges::find(seen, d) != seen.end());

        call_count = 0;
        seen.clear();
        auto pred_none = Value::create(Function{std::make_shared<Builtin>(
            [&](builtin_args_t args) {
                ++call_count;
                seen.push_back(args.at(0));
                return Value::create(false);
            },
            "pred_none", Builtin::Arity{.min = 1, .max = 1})});

        auto res_none = fn->call({arr, pred_none});
        require_array_eq(res_none, {});
        CHECK(call_count >= 1);
        REQUIRE(!seen.empty());
        CHECK(std::ranges::all_of(seen, [&](const Value_Ptr& v) {
            return v == a;
        }));

        call_count = 0;
        auto empty = Value::create(Array{});
        auto res_empty = fn->call({empty, pred_all});
        require_array_eq(res_empty, {});
        CHECK(call_count == 0);
    }

    SECTION("drop_while semantics")
    {
        auto fn = lookup(table, "drop_while");
        auto a = Value::create(0_f);
        auto b = Value::create(1_f);
        auto c = Value::create(2_f);
        auto d = Value::create(3_f);
        auto arr = Value::create(Array{a, b, c, d});

        std::size_t call_count = 0;
        std::vector<Value_Ptr> seen;
        auto pred = Value::create(Function{std::make_shared<Builtin>(
            [&](builtin_args_t args) {
                ++call_count;
                seen.push_back(args.at(0));
                return Value::create(args.at(0)->raw_get<Int>() < 2_f);
            },
            "pred", Builtin::Arity{.min = 1, .max = 1})});

        auto res = fn->call({arr, pred});
        require_array_eq(res, {c, d});
        CHECK(call_count >= 3);
        REQUIRE(seen.size() >= 3);
        CHECK(std::ranges::find(seen, a) != seen.end());
        CHECK(std::ranges::find(seen, b) != seen.end());
        CHECK(std::ranges::find(seen, c) != seen.end());
        CHECK(std::ranges::find(seen, d) == seen.end());

        call_count = 0;
        seen.clear();
        auto pred_all = Value::create(Function{std::make_shared<Builtin>(
            [&](builtin_args_t args) {
                ++call_count;
                seen.push_back(args.at(0));
                return Value::create(true);
            },
            "pred_all", Builtin::Arity{.min = 1, .max = 1})});

        auto res_all = fn->call({arr, pred_all});
        require_array_eq(res_all, {});
        CHECK(call_count >= 4);
        CHECK(std::ranges::find(seen, a) != seen.end());
        CHECK(std::ranges::find(seen, b) != seen.end());
        CHECK(std::ranges::find(seen, c) != seen.end());
        CHECK(std::ranges::find(seen, d) != seen.end());

        call_count = 0;
        seen.clear();
        auto pred_none = Value::create(Function{std::make_shared<Builtin>(
            [&](builtin_args_t args) {
                ++call_count;
                seen.push_back(args.at(0));
                return Value::create(false);
            },
            "pred_none", Builtin::Arity{.min = 1, .max = 1})});

        auto res_none = fn->call({arr, pred_none});
        require_array_eq(res_none, {a, b, c, d});
        CHECK(call_count >= 1);
        REQUIRE(!seen.empty());
        CHECK(std::ranges::all_of(seen, [&](const Value_Ptr& v) {
            return v == a;
        }));

        call_count = 0;
        auto empty = Value::create(Array{});
        auto res_empty = fn->call({empty, pred_all});
        require_array_eq(res_empty, {});
        CHECK(call_count == 0);
    }

    SECTION("while predicate error propagates")
    {
        auto take_fn = lookup(table, "take_while");
        auto drop_fn = lookup(table, "drop_while");
        auto arr = Value::create(Array{Value::create(1_f)});
        auto boom = Value::create(Function{std::make_shared<Builtin>(
            [](builtin_args_t) -> Value_Ptr {
                throw Frost_Recoverable_Error{"kaboom"};
            },
            "boom", Builtin::Arity{.min = 1, .max = 1})});

        CHECK_THROWS_WITH(take_fn->call({arr, boom}),
                          ContainsSubstring("kaboom"));
        CHECK_THROWS_WITH(drop_fn->call({arr, boom}),
                          ContainsSubstring("kaboom"));
    }

    SECTION("chunk_by semantics")
    {
        auto fn = lookup(table, "chunk_by");
        auto a = Value::create(1_f);
        auto b = Value::create(1_f);
        auto c = Value::create(2_f);
        auto d = Value::create(2_f);
        auto e = Value::create(3_f);
        auto f = Value::create(1_f);
        auto g = Value::create(1_f);
        auto arr = Value::create(Array{a, b, c, d, e, f, g});

        auto eq_pred = Value::create(Function{std::make_shared<Builtin>(
            [&](builtin_args_t args) {
                return Value::create(args.at(0)->raw_get<Int>()
                                     == args.at(1)->raw_get<Int>());
            },
            "eq", Builtin::Arity{.min = 2, .max = 2})});

        auto res = fn->call({arr, eq_pred});
        REQUIRE(res->is<Array>());
        const auto& outer = res->raw_get<Array>();
        REQUIRE(outer.size() == 4);
        require_array_eq(outer.at(0), {a, b});
        require_array_eq(outer.at(1), {c, d});
        require_array_eq(outer.at(2), {e});
        require_array_eq(outer.at(3), {f, g});

        auto always_true = Value::create(Function{std::make_shared<Builtin>(
            [](builtin_args_t) {
                return Value::create(true);
            },
            "true", Builtin::Arity{.min = 2, .max = 2})});
        auto res_true = fn->call({arr, always_true});
        REQUIRE(res_true->is<Array>());
        const auto& outer_true = res_true->raw_get<Array>();
        REQUIRE(outer_true.size() == 1);
        require_array_eq(outer_true.at(0), {a, b, c, d, e, f, g});

        auto always_false = Value::create(Function{std::make_shared<Builtin>(
            [](builtin_args_t) {
                return Value::create(false);
            },
            "false", Builtin::Arity{.min = 2, .max = 2})});
        auto res_false = fn->call({arr, always_false});
        REQUIRE(res_false->is<Array>());
        const auto& outer_false = res_false->raw_get<Array>();
        REQUIRE(outer_false.size() == 7);
        require_array_eq(outer_false.at(0), {a});
        require_array_eq(outer_false.at(1), {b});
        require_array_eq(outer_false.at(2), {c});
        require_array_eq(outer_false.at(3), {d});
        require_array_eq(outer_false.at(4), {e});
        require_array_eq(outer_false.at(5), {f});
        require_array_eq(outer_false.at(6), {g});

        std::size_t call_count = 0;
        auto empty = Value::create(Array{});
        auto count_pred = Value::create(Function{std::make_shared<Builtin>(
            [&](builtin_args_t) {
                ++call_count;
                return Value::create(true);
            },
            "count", Builtin::Arity{.min = 2, .max = 2})});
        auto res_empty = fn->call({empty, count_pred});
        require_array_eq(res_empty, {});
        CHECK(call_count == 0);
    }

    SECTION("transform semantics")
    {
        auto fn = lookup(table, "transform");
        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto arr = Value::create(Array{a, b});

        auto op = make_builtin(
            [](builtin_args_t args) {
                return Value::create(args.at(0)->raw_get<Int>() * 2_f);
            },
            "op", Builtin::Arity{.min = 1, .max = 1});

        auto res = fn->call({arr, Value::create(Function{op})});
        REQUIRE(res->is<Array>());
        const auto& out = res->raw_get<Array>();
        REQUIRE(out.size() == 2);
        CHECK(out.at(0)->raw_get<Int>() == 2_f);
        CHECK(out.at(1)->raw_get<Int>() == 4_f);

        auto key_a = Value::create("a"s);
        auto key_b = Value::create("b"s);
        auto map = Value::create(Map{{key_a, a}, {key_b, b}});

        auto map_op = make_builtin(
            [](builtin_args_t args) {
                return Value::create(
                    Map{{args.at(0),
                         Value::create(args.at(1)->raw_get<Int>() * 2_f)}});
            },
            "map_op", Builtin::Arity{.min = 2, .max = 2});

        auto map_res = fn->call({map, Value::create(Function{map_op})});
        REQUIRE(map_res->is<Map>());
        const auto& out_map = map_res->raw_get<Map>();
        REQUIRE(out_map.size() == 2);
        CHECK(out_map.at(key_a)->raw_get<Int>() == 2_f);
        CHECK(out_map.at(key_b)->raw_get<Int>() == 4_f);
    }

    SECTION("transform map errors")
    {
        auto fn = lookup(table, "transform");
        auto key = Value::create("k"s);
        auto key2 = Value::create("j"s);
        auto map = Value::create(
            Map{{key, Value::create(1_f)}, {key2, Value::create(2_f)}});

        auto bad_op = make_builtin(
            [](builtin_args_t) {
                return Value::create(1_f);
            },
            "bad", Builtin::Arity{.min = 2, .max = 2});

        CHECK_THROWS_WITH(fn->call({map, Value::create(Function{bad_op})}),
                          ContainsSubstring("Builtin transform"));

        auto collision_key = Value::create("x"s);
        auto collide_op = make_builtin(
            [collision_key](builtin_args_t) {
                return Value::create(Map{{collision_key, Value::create(1_f)}});
            },
            "collide", Builtin::Arity{.min = 2, .max = 2});

        CHECK_THROWS_WITH(fn->call({map, Value::create(Function{collide_op})}),
                          ContainsSubstring("key collision"));
    }

    SECTION("select semantics")
    {
        auto fn = lookup(table, "select");
        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto c = Value::create(3_f);
        auto arr = Value::create(Array{a, b, c});

        auto pred = make_builtin(
            [](builtin_args_t args) {
                return Value::create(args.at(0)->raw_get<Int>() > 1_f);
            },
            "pred", Builtin::Arity{.min = 1, .max = 1});

        auto res = fn->call({arr, Value::create(Function{pred})});
        require_array_eq(res, {b, c});

        auto key_a = Value::create("a"s);
        auto key_b = Value::create("b"s);
        auto map = Value::create(Map{{key_a, a}, {key_b, b}});

        auto map_pred = make_builtin(
            [](builtin_args_t args) {
                return Value::create(args.at(1)->raw_get<Int>() == 2_f);
            },
            "pred", Builtin::Arity{.min = 2, .max = 2});

        auto map_res = fn->call({map, Value::create(Function{map_pred})});
        REQUIRE(map_res->is<Map>());
        const auto& out = map_res->raw_get<Map>();
        REQUIRE(out.size() == 1);
        CHECK(out.begin()->first == key_b);
        CHECK(out.begin()->second == b);
    }

    SECTION("fold semantics")
    {
        auto fn = lookup(table, "fold");
        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto c = Value::create(3_f);
        auto arr = Value::create(Array{a, b, c});

        auto op = make_builtin(
            [](builtin_args_t args) {
                return Value::create(args.at(0)->raw_get<Int>()
                                     + args.at(1)->raw_get<Int>());
            },
            "op", Builtin::Arity{.min = 2, .max = 2});

        auto res = fn->call({arr, Value::create(Function{op})});
        CHECK(res->raw_get<Int>() == 6_f);

        auto init = Value::create(10_f);
        auto res_init = fn->call({arr, Value::create(Function{op}), init});
        CHECK(res_init->raw_get<Int>() == 16_f);

        auto empty = Value::create(Array{});
        auto res_empty = fn->call({empty, Value::create(Function{op})});
        CHECK(res_empty == Value::null());
        auto res_empty_init =
            fn->call({empty, Value::create(Function{op}), init});
        CHECK(res_empty_init == init);

        auto key_a = Value::create("a"s);
        auto key_b = Value::create("b"s);
        auto map = Value::create(Map{{key_a, a}, {key_b, b}});

        auto map_op = make_builtin(
            [](builtin_args_t args) {
                return Value::create(args.at(0)->raw_get<Int>()
                                     + args.at(2)->raw_get<Int>());
            },
            "op", Builtin::Arity{.min = 3, .max = 3});

        auto res_map = fn->call({map, Value::create(Function{map_op}), init});
        CHECK(res_map->raw_get<Int>() == 13_f);
    }

    SECTION("fold map requires init")
    {
        auto fn = lookup(table, "fold");
        auto map =
            Value::create(Map{{Value::create("a"s), Value::create(1_f)}});
        auto op = make_builtin(
            [](builtin_args_t args) {
                return args.at(0);
            },
            "op", Builtin::Arity{.min = 3, .max = 3});

        CHECK_THROWS_WITH(fn->call({map, Value::create(Function{op})}),
                          ContainsSubstring("Map reduction requires init"));
    }

    SECTION("group_by semantics")
    {
        auto fn = lookup(table, "group_by");
        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto c = Value::create(3_f);
        auto d = Value::create(4_f);
        auto e = Value::create(5_f);
        auto arr = Value::create(Array{a, b, c, d, e});

        auto callable = mock::Mock_Callable::make();
        auto fn_val = Value::create(Function{callable});

        auto k1 = Value::create(1_f);
        auto k2 = Value::create(0_f);
        auto k3 = Value::create(1_f);
        auto k4 = Value::create(0_f);
        auto k5 = Value::create(1_f);

        trompeloeil::sequence seq;
        REQUIRE_CALL(*callable, call(_))
            .LR_WITH(_1.size() == 1 && _1[0] == a)
            .IN_SEQUENCE(seq)
            .RETURN(k1);
        REQUIRE_CALL(*callable, call(_))
            .LR_WITH(_1.size() == 1 && _1[0] == b)
            .IN_SEQUENCE(seq)
            .RETURN(k2);
        REQUIRE_CALL(*callable, call(_))
            .LR_WITH(_1.size() == 1 && _1[0] == c)
            .IN_SEQUENCE(seq)
            .RETURN(k3);
        REQUIRE_CALL(*callable, call(_))
            .LR_WITH(_1.size() == 1 && _1[0] == d)
            .IN_SEQUENCE(seq)
            .RETURN(k4);
        REQUIRE_CALL(*callable, call(_))
            .LR_WITH(_1.size() == 1 && _1[0] == e)
            .IN_SEQUENCE(seq)
            .RETURN(k5);

        auto res = fn->call({arr, fn_val});
        REQUIRE(res->is<Map>());
        const auto& out = res->raw_get<Map>();
        REQUIRE(out.size() == 2);

        auto odd_key = Value::create(1_f);
        auto even_key = Value::create(0_f);

        auto odd_it = out.find(odd_key);
        REQUIRE(odd_it != out.end());
        require_array_eq(odd_it->second, {a, c, e});

        auto even_it = out.find(even_key);
        REQUIRE(even_it != out.end());
        require_array_eq(even_it->second, {b, d});
    }

    SECTION("group_by empty array returns empty map")
    {
        auto fn = lookup(table, "group_by");
        auto empty = Value::create(Array{});
        auto callable = mock::Mock_Callable::make();
        auto fn_val = Value::create(Function{callable});

        FORBID_CALL(*callable, call(_));

        auto res = fn->call({empty, fn_val});
        REQUIRE(res->is<Map>());
        CHECK(res->raw_get<Map>().empty());
    }

    SECTION("group_by rejects non-primitive keys")
    {
        auto fn = lookup(table, "group_by");
        auto arr = Value::create(Array{Value::create(1_f)});

        {
            auto callable = mock::Mock_Callable::make();
            auto fn_val = Value::create(Function{callable});
            REQUIRE_CALL(*callable, call(_))
                .RETURN(Value::create(Array{Value::create(1_f)}));

            CHECK_THROWS_MATCHES(
                fn->call({arr, fn_val}), Frost_Recoverable_Error,
                MessageMatches(ContainsSubstring("Array")));
        }

        {
            auto callable = mock::Mock_Callable::make();
            auto fn_val = Value::create(Function{callable});
            REQUIRE_CALL(*callable, call(_))
                .RETURN(Value::create(Map{}));

            CHECK_THROWS_MATCHES(
                fn->call({arr, fn_val}), Frost_Recoverable_Error,
                MessageMatches(ContainsSubstring("Map")));
        }
    }

    SECTION("group_by propagates key errors")
    {
        auto fn = lookup(table, "group_by");
        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto c = Value::create(3_f);
        auto arr = Value::create(Array{a, b, c});

        auto callable = mock::Mock_Callable::make();
        auto fn_val = Value::create(Function{callable});

        trompeloeil::sequence seq;
        REQUIRE_CALL(*callable, call(_))
            .LR_WITH(_1.size() == 1 && _1[0] == a)
            .IN_SEQUENCE(seq)
            .RETURN(Value::create(0_f));
        REQUIRE_CALL(*callable, call(_))
            .LR_WITH(_1.size() == 1 && _1[0] == b)
            .IN_SEQUENCE(seq)
            .THROW(Frost_Recoverable_Error{"boom"});

        CHECK_THROWS_WITH(fn->call({arr, fn_val}), ContainsSubstring("boom"));
    }

    SECTION("count_by semantics")
    {
        auto fn = lookup(table, "count_by");
        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto c = Value::create(3_f);
        auto d = Value::create(4_f);
        auto e = Value::create(5_f);
        auto arr = Value::create(Array{a, b, c, d, e});

        auto callable = mock::Mock_Callable::make();
        auto fn_val = Value::create(Function{callable});

        trompeloeil::sequence seq;
        REQUIRE_CALL(*callable, call(_))
            .LR_WITH(_1.size() == 1 && _1[0] == a)
            .IN_SEQUENCE(seq)
            .RETURN(Value::create(1_f));
        REQUIRE_CALL(*callable, call(_))
            .LR_WITH(_1.size() == 1 && _1[0] == b)
            .IN_SEQUENCE(seq)
            .RETURN(Value::create(0_f));
        REQUIRE_CALL(*callable, call(_))
            .LR_WITH(_1.size() == 1 && _1[0] == c)
            .IN_SEQUENCE(seq)
            .RETURN(Value::create(1_f));
        REQUIRE_CALL(*callable, call(_))
            .LR_WITH(_1.size() == 1 && _1[0] == d)
            .IN_SEQUENCE(seq)
            .RETURN(Value::create(0_f));
        REQUIRE_CALL(*callable, call(_))
            .LR_WITH(_1.size() == 1 && _1[0] == e)
            .IN_SEQUENCE(seq)
            .RETURN(Value::create(1_f));

        auto res = fn->call({arr, fn_val});
        REQUIRE(res->is<Map>());
        const auto& out = res->raw_get<Map>();
        REQUIRE(out.size() == 2);

        auto odd_key = Value::create(1_f);
        auto even_key = Value::create(0_f);

        auto odd_it = out.find(odd_key);
        REQUIRE(odd_it != out.end());
        REQUIRE(odd_it->second->is<Int>());
        CHECK(odd_it->second->raw_get<Int>() == 3_f);

        auto even_it = out.find(even_key);
        REQUIRE(even_it != out.end());
        REQUIRE(even_it->second->is<Int>());
        CHECK(even_it->second->raw_get<Int>() == 2_f);
    }

    SECTION("count_by empty array returns empty map")
    {
        auto fn = lookup(table, "count_by");
        auto empty = Value::create(Array{});
        auto callable = mock::Mock_Callable::make();
        auto fn_val = Value::create(Function{callable});

        FORBID_CALL(*callable, call(_));

        auto res = fn->call({empty, fn_val});
        REQUIRE(res->is<Map>());
        CHECK(res->raw_get<Map>().empty());
    }

    SECTION("count_by rejects non-primitive keys")
    {
        auto fn = lookup(table, "count_by");
        auto arr = Value::create(Array{Value::create(1_f)});

        {
            auto callable = mock::Mock_Callable::make();
            auto fn_val = Value::create(Function{callable});
            REQUIRE_CALL(*callable, call(_))
                .RETURN(Value::create(Array{Value::create(1_f)}));

            CHECK_THROWS_MATCHES(
                fn->call({arr, fn_val}), Frost_Recoverable_Error,
                MessageMatches(ContainsSubstring("Array")));
        }

        {
            auto callable = mock::Mock_Callable::make();
            auto fn_val = Value::create(Function{callable});
            REQUIRE_CALL(*callable, call(_))
                .RETURN(Value::create(Map{}));

            CHECK_THROWS_MATCHES(
                fn->call({arr, fn_val}), Frost_Recoverable_Error,
                MessageMatches(ContainsSubstring("Map")));
        }
    }

    SECTION("count_by propagates key errors")
    {
        auto fn = lookup(table, "count_by");
        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto c = Value::create(3_f);
        auto arr = Value::create(Array{a, b, c});

        auto callable = mock::Mock_Callable::make();
        auto fn_val = Value::create(Function{callable});

        trompeloeil::sequence seq;
        REQUIRE_CALL(*callable, call(_))
            .LR_WITH(_1.size() == 1 && _1[0] == a)
            .IN_SEQUENCE(seq)
            .RETURN(Value::create(0_f));
        REQUIRE_CALL(*callable, call(_))
            .LR_WITH(_1.size() == 1 && _1[0] == b)
            .IN_SEQUENCE(seq)
            .THROW(Frost_Recoverable_Error{"boom"});

        CHECK_THROWS_WITH(fn->call({arr, fn_val}), ContainsSubstring("boom"));
    }

    SECTION("scan semantics")
    {
        auto fn = lookup(table, "scan");
        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto c = Value::create(3_f);
        auto arr = Value::create(Array{a, b, c});

        auto callable = mock::Mock_Callable::make();
        auto fn_val = Value::create(Function{callable});

        trompeloeil::sequence seq;
        REQUIRE_CALL(*callable, call(_))
            .LR_WITH(_1.size() == 2 && _1[0] == a && _1[1] == b)
            .IN_SEQUENCE(seq)
            .RETURN(Value::create(3_f));
        REQUIRE_CALL(*callable, call(_))
            .LR_WITH(_1.size() == 2 && require_int(_1[0]) == 3_f
                     && _1[1] == c)
            .IN_SEQUENCE(seq)
            .RETURN(Value::create(6_f));

        auto res = fn->call({arr, fn_val});
        REQUIRE(res->is<Array>());
        const auto& out = res->raw_get<Array>();
        REQUIRE(out.size() == 3);
        CHECK(out.at(0) == a);
        CHECK(out.at(1)->raw_get<Int>() == 3_f);
        CHECK(out.at(2)->raw_get<Int>() == 6_f);
    }

    SECTION("scan empty array returns empty array")
    {
        auto fn = lookup(table, "scan");
        auto empty = Value::create(Array{});
        auto callable = mock::Mock_Callable::make();
        auto fn_val = Value::create(Function{callable});

        FORBID_CALL(*callable, call(_));

        auto res = fn->call({empty, fn_val});
        REQUIRE(res->is<Array>());
        CHECK(res->raw_get<Array>().empty());
    }

    SECTION("scan single element returns same element")
    {
        auto fn = lookup(table, "scan");
        auto a = Value::create(1_f);
        auto arr = Value::create(Array{a});
        auto callable = mock::Mock_Callable::make();
        auto fn_val = Value::create(Function{callable});

        FORBID_CALL(*callable, call(_));

        auto res = fn->call({arr, fn_val});
        REQUIRE(res->is<Array>());
        const auto& out = res->raw_get<Array>();
        REQUIRE(out.size() == 1);
        CHECK(out.at(0) == a);
    }

    SECTION("scan propagates reducer errors")
    {
        auto fn = lookup(table, "scan");
        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto c = Value::create(3_f);
        auto arr = Value::create(Array{a, b, c});

        auto callable = mock::Mock_Callable::make();
        auto fn_val = Value::create(Function{callable});

        trompeloeil::sequence seq;
        REQUIRE_CALL(*callable, call(_))
            .LR_WITH(_1.size() == 2 && _1[0] == a && _1[1] == b)
            .IN_SEQUENCE(seq)
            .THROW(Frost_Recoverable_Error{"boom"});

        CHECK_THROWS_WITH(fn->call({arr, fn_val}), ContainsSubstring("boom"));
    }

    SECTION("any/all/none default predicate semantics")
    {
        auto any_fn = lookup(table, "any");
        auto all_fn = lookup(table, "all");
        auto none_fn = lookup(table, "none");

        auto v_null = Value::null();
        auto v_zero = Value::create(0_f);
        auto v_one = Value::create(1_f);
        auto arr = Value::create(Array{v_null, v_zero, v_one});

        CHECK(any_fn->call({arr})->get<Bool>().value() == true);
        CHECK(all_fn->call({arr})->get<Bool>().value() == false);
        CHECK(none_fn->call({arr})->get<Bool>().value() == false);

        auto empty = Value::create(Array{});
        CHECK(any_fn->call({empty})->get<Bool>().value() == false);
        CHECK(all_fn->call({empty})->get<Bool>().value() == true);
        CHECK(none_fn->call({empty})->get<Bool>().value() == true);

        auto all_falsy = Value::create(Array{Value::null(), Value::create(false)});
        CHECK(any_fn->call({all_falsy})->get<Bool>().value() == false);
        CHECK(all_fn->call({all_falsy})->get<Bool>().value() == false);
        CHECK(none_fn->call({all_falsy})->get<Bool>().value() == true);
    }

    SECTION("any/all/none predicate semantics")
    {
        auto any_fn = lookup(table, "any");
        auto all_fn = lookup(table, "all");
        auto none_fn = lookup(table, "none");

        auto a = Value::create(0_f);
        auto b = Value::create(1_f);
        auto c = Value::create(2_f);
        auto arr = Value::create(Array{a, b, c});

        auto pred_lt_two = make_builtin(
            [](builtin_args_t args) {
                return Value::create(args.at(0)->raw_get<Int>() < 2_f);
            },
            "pred_lt_two", Builtin::Arity{.min = 1, .max = 1});

        CHECK(any_fn->call({arr, Value::create(Function{pred_lt_two})})
                  ->get<Bool>()
                  .value()
              == true);
        CHECK(all_fn->call({arr, Value::create(Function{pred_lt_two})})
                  ->get<Bool>()
                  .value()
              == false);
        CHECK(none_fn->call({arr, Value::create(Function{pred_lt_two})})
                  ->get<Bool>()
                  .value()
              == false);

        std::size_t call_count = 0;
        auto empty = Value::create(Array{});
        auto count_pred = make_builtin(
            [&](builtin_args_t) {
                ++call_count;
                return Value::create(true);
            },
            "count", Builtin::Arity{.min = 1, .max = 1});

        CHECK(any_fn->call({empty, Value::create(Function{count_pred})})
                  ->get<Bool>()
                  .value()
              == false);
        CHECK(all_fn->call({empty, Value::create(Function{count_pred})})
                  ->get<Bool>()
                  .value()
              == true);
        CHECK(none_fn->call({empty, Value::create(Function{count_pred})})
                  ->get<Bool>()
                  .value()
              == true);
        CHECK(call_count == 0);
    }

    SECTION("any/all/none predicate all-true and all-false")
    {
        auto any_fn = lookup(table, "any");
        auto all_fn = lookup(table, "all");
        auto none_fn = lookup(table, "none");

        auto arr = Value::create(Array{Value::create(1_f), Value::create(2_f)});

        auto pred_true = make_builtin(
            [](builtin_args_t) { return Value::create(true); }, "pred_true",
            Builtin::Arity{.min = 1, .max = 1});
        auto pred_false = make_builtin(
            [](builtin_args_t) { return Value::create(false); }, "pred_false",
            Builtin::Arity{.min = 1, .max = 1});

        CHECK(any_fn->call({arr, Value::create(Function{pred_true})})
                  ->get<Bool>()
                  .value()
              == true);
        CHECK(all_fn->call({arr, Value::create(Function{pred_true})})
                  ->get<Bool>()
                  .value()
              == true);
        CHECK(none_fn->call({arr, Value::create(Function{pred_true})})
                  ->get<Bool>()
                  .value()
              == false);

        CHECK(any_fn->call({arr, Value::create(Function{pred_false})})
                  ->get<Bool>()
                  .value()
              == false);
        CHECK(all_fn->call({arr, Value::create(Function{pred_false})})
                  ->get<Bool>()
                  .value()
              == false);
        CHECK(none_fn->call({arr, Value::create(Function{pred_false})})
                  ->get<Bool>()
                  .value()
              == true);
    }

    SECTION("any/all/none predicate non-bool results")
    {
        auto any_fn = lookup(table, "any");
        auto all_fn = lookup(table, "all");
        auto none_fn = lookup(table, "none");

        auto arr = Value::create(Array{Value::create(1_f), Value::create(2_f)});

        auto pred_null = make_builtin(
            [](builtin_args_t) { return Value::null(); }, "pred_null",
            Builtin::Arity{.min = 1, .max = 1});
        auto pred_int = make_builtin(
            [](builtin_args_t) { return Value::create(0_f); }, "pred_int",
            Builtin::Arity{.min = 1, .max = 1});

        CHECK(any_fn->call({arr, Value::create(Function{pred_null})})
                  ->get<Bool>()
                  .value()
              == false);
        CHECK(all_fn->call({arr, Value::create(Function{pred_null})})
                  ->get<Bool>()
                  .value()
              == false);
        CHECK(none_fn->call({arr, Value::create(Function{pred_null})})
                  ->get<Bool>()
                  .value()
              == true);

        CHECK(any_fn->call({arr, Value::create(Function{pred_int})})
                  ->get<Bool>()
                  .value()
              == true);
        CHECK(all_fn->call({arr, Value::create(Function{pred_int})})
                  ->get<Bool>()
                  .value()
              == true);
        CHECK(none_fn->call({arr, Value::create(Function{pred_int})})
                  ->get<Bool>()
                  .value()
              == false);
    }

    SECTION("any/all/none short-circuit")
    {
        auto any_fn = lookup(table, "any");
        auto all_fn = lookup(table, "all");
        auto none_fn = lookup(table, "none");

        auto a = Value::create(0_f);
        auto b = Value::create(1_f);
        auto c = Value::create(2_f);
        auto arr = Value::create(Array{a, b, c});

        std::size_t call_count = 0;
        std::vector<Value_Ptr> seen;
        auto pred_any = make_builtin(
            [&](builtin_args_t args) {
                ++call_count;
                seen.push_back(args.at(0));
                return Value::create(args.at(0)->raw_get<Int>() > 0_f);
            },
            "pred_any", Builtin::Arity{.min = 1, .max = 1});

        CHECK(any_fn->call({arr, Value::create(Function{pred_any})})
                  ->get<Bool>()
                  .value()
              == true);
        CHECK(call_count == 2);
        REQUIRE(seen.size() == 2);
        CHECK(seen.at(0) == a);
        CHECK(seen.at(1) == b);

        call_count = 0;
        seen.clear();
        auto pred_all = make_builtin(
            [&](builtin_args_t args) {
                ++call_count;
                seen.push_back(args.at(0));
                return Value::create(args.at(0)->raw_get<Int>() == 0_f);
            },
            "pred_all", Builtin::Arity{.min = 1, .max = 1});

        CHECK(all_fn->call({arr, Value::create(Function{pred_all})})
                  ->get<Bool>()
                  .value()
              == false);
        CHECK(call_count == 2);
        REQUIRE(seen.size() == 2);
        CHECK(seen.at(0) == a);
        CHECK(seen.at(1) == b);

        call_count = 0;
        seen.clear();
        auto pred_none = make_builtin(
            [&](builtin_args_t args) {
                ++call_count;
                seen.push_back(args.at(0));
                return Value::create(args.at(0)->raw_get<Int>() == 1_f);
            },
            "pred_none", Builtin::Arity{.min = 1, .max = 1});

        CHECK(none_fn->call({arr, Value::create(Function{pred_none})})
                  ->get<Bool>()
                  .value()
              == false);
        CHECK(call_count == 2);
        REQUIRE(seen.size() == 2);
        CHECK(seen.at(0) == a);
        CHECK(seen.at(1) == b);
    }

    SECTION("any/all/none predicate error propagates")
    {
        auto any_fn = lookup(table, "any");
        auto all_fn = lookup(table, "all");
        auto none_fn = lookup(table, "none");

        auto arr = Value::create(Array{Value::create(1_f)});
        auto boom = make_builtin(
            [](builtin_args_t) -> Value_Ptr {
                throw Frost_Recoverable_Error{"kaboom"};
            },
            "boom", Builtin::Arity{.min = 1, .max = 1});

        CHECK_THROWS_WITH(
            any_fn->call({arr, Value::create(Function{boom})}),
            ContainsSubstring("kaboom"));
        CHECK_THROWS_WITH(
            all_fn->call({arr, Value::create(Function{boom})}),
            ContainsSubstring("kaboom"));
        CHECK_THROWS_WITH(
            none_fn->call({arr, Value::create(Function{boom})}),
            ContainsSubstring("kaboom"));
    }

    SECTION("sorted semantics")
    {
        auto fn = lookup(table, "sorted");
        auto a = Value::create(3_f);
        auto b = Value::create(1_f);
        auto c = Value::create(2_f);
        auto arr = Value::create(Array{a, b, c});

        require_array_eq(fn->call({arr}), {b, c, a});

        const auto& orig = arr->raw_get<Array>();
        REQUIRE(orig.size() == 3);
        CHECK(orig.at(0) == a);
        CHECK(orig.at(1) == b);
        CHECK(orig.at(2) == c);

        auto desc = make_builtin(
            [](builtin_args_t args) {
                return Value::greater_than(args.at(0), args.at(1));
            },
            "desc", Builtin::Arity{.min = 2, .max = 2});

        require_array_eq(
            fn->call({arr, Value::create(Function{desc})}), {a, c, b});

        auto empty = Value::create(Array{});
        require_array_eq(fn->call({empty}), {});

        auto single = Value::create(Array{a});
        require_array_eq(fn->call({single}), {a});

        auto dup_arr = Value::create(Array{Value::create(2_f), Value::create(1_f),
                                           Value::create(1_f), Value::create(3_f)});
        auto dup_sorted = fn->call({dup_arr});
        REQUIRE(dup_sorted->is<Array>());
        const auto& dup_out = dup_sorted->raw_get<Array>();
        REQUIRE(dup_out.size() == 4);
        CHECK(dup_out.at(0)->raw_get<Int>() == 1_f);
        CHECK(dup_out.at(1)->raw_get<Int>() == 1_f);
        CHECK(dup_out.at(2)->raw_get<Int>() == 2_f);
        CHECK(dup_out.at(3)->raw_get<Int>() == 3_f);

        auto s1 = Value::create("b"s);
        auto s2 = Value::create("a"s);
        auto s3 = Value::create("c"s);
        auto str_arr = Value::create(Array{s1, s2, s3});
        require_array_eq(fn->call({str_arr}), {s2, s1, s3});

        auto truthy_int_cmp = make_builtin(
            [](builtin_args_t args) {
                if (args.at(0)->raw_get<Int>() < args.at(1)->raw_get<Int>())
                    return Value::create(1_f);
                return Value::null();
            },
            "truthy_int_cmp", Builtin::Arity{.min = 2, .max = 2});

        require_array_eq(
            fn->call({arr, Value::create(Function{truthy_int_cmp})}), {b, c, a});
    }

    SECTION("sorted predicate error propagates")
    {
        auto fn = lookup(table, "sorted");
        auto arr =
            Value::create(Array{Value::create(1_f), Value::create(0_f)});
        auto boom = make_builtin(
            [](builtin_args_t) -> Value_Ptr {
                throw Frost_Recoverable_Error{"kaboom"};
            },
            "boom", Builtin::Arity{.min = 2, .max = 2});

        CHECK_THROWS_WITH(
            fn->call({arr, Value::create(Function{boom})}),
            ContainsSubstring("kaboom"));
    }

    SECTION("sorted rejects non-comparable elements")
    {
        auto fn = lookup(table, "sorted");
        auto mixed = Value::create(Array{Value::create(1_f), Value::create("a"s)});

        CHECK_THROWS_WITH(fn->call({mixed}),
                          ContainsSubstring("compare incompatible types"));
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
