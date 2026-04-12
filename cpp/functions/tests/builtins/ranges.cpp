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
Function make_builtin(Fn fn, std::string name)
{
    return std::make_shared<Builtin>(std::move(fn), std::move(name));
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
        "stride", "take", "drop", "tail", "drop_tail", "slide", "chunk",
    };
    const std::vector<std::string> unary_names{
        "reverse",
    };
    const std::vector<std::string> variadic_names{
        "zip",
        "xprod",
    };
    const std::vector<std::string> pred_names{
        "take_while", "drop_while", "chunk_by", "group_by", "count_by",
        "scan",       "partition",  "sort_by",  "find",
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

    SECTION("tail semantics")
    {
        auto fn = lookup(table, "tail");
        auto a = Value::create(0_f);
        auto b = Value::create(1_f);
        auto c = Value::create(2_f);
        auto d = Value::create(3_f);
        auto e = Value::create(4_f);
        auto arr = Value::create(Array{a, b, c, d, e});

        require_array_eq(fn->call({arr, Value::create(0_f)}), {});
        require_array_eq(fn->call({arr, Value::create(2_f)}), {d, e});
        require_array_eq(fn->call({arr, Value::create(10_f)}), {a, b, c, d, e});
        require_array_eq(fn->call({arr, Value::create(5_f)}), {a, b, c, d, e});
        require_array_eq(fn->call({arr, Value::create(4_f)}), {b, c, d, e});

        auto empty_arr = Value::create(Array{});
        require_array_eq(fn->call({empty_arr, Value::create(2_f)}), {});

        CHECK_THROWS_MATCHES(fn->call({arr, Value::create(-1_f)}),
                             Frost_User_Error,
                             MessageMatches(ContainsSubstring("Function tail")
                                            && ContainsSubstring(">=0")));
    }

    SECTION("drop_tail semantics")
    {
        auto fn = lookup(table, "drop_tail");
        auto a = Value::create(0_f);
        auto b = Value::create(1_f);
        auto c = Value::create(2_f);
        auto d = Value::create(3_f);
        auto e = Value::create(4_f);
        auto arr = Value::create(Array{a, b, c, d, e});

        require_array_eq(fn->call({arr, Value::create(0_f)}), {a, b, c, d, e});
        require_array_eq(fn->call({arr, Value::create(2_f)}), {a, b, c});
        require_array_eq(fn->call({arr, Value::create(10_f)}), {});
        require_array_eq(fn->call({arr, Value::create(5_f)}), {});
        require_array_eq(fn->call({arr, Value::create(4_f)}), {a});

        auto empty_arr = Value::create(Array{});
        require_array_eq(fn->call({empty_arr, Value::create(2_f)}), {});

        CHECK_THROWS_MATCHES(
            fn->call({arr, Value::create(-1_f)}), Frost_User_Error,
            MessageMatches(ContainsSubstring("Function drop_tail")
                           && ContainsSubstring(">=0")));
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
            "pred")});

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
            "pred_all")});

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
            "pred_none")});

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
            "pred")});

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
            "pred_all")});

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
            "pred_none")});

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
            "boom")});

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
            "eq")});

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
            "true")});
        auto res_true = fn->call({arr, always_true});
        REQUIRE(res_true->is<Array>());
        const auto& outer_true = res_true->raw_get<Array>();
        REQUIRE(outer_true.size() == 1);
        require_array_eq(outer_true.at(0), {a, b, c, d, e, f, g});

        auto always_false = Value::create(Function{std::make_shared<Builtin>(
            [](builtin_args_t) {
                return Value::create(false);
            },
            "false")});
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
            "count")});
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
            "op");

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
            "map_op");

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
            "bad");

        CHECK_THROWS_WITH(fn->call({map, Value::create(Function{bad_op})}),
                          ContainsSubstring("Builtin transform"));

        auto collision_key = Value::create("x"s);
        auto collide_op = make_builtin(
            [collision_key](builtin_args_t) {
                return Value::create(Map{{collision_key, Value::create(1_f)}});
            },
            "collide");

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
            "pred");

        auto res = fn->call({arr, Value::create(Function{pred})});
        require_array_eq(res, {b, c});

        auto key_a = Value::create("a"s);
        auto key_b = Value::create("b"s);
        auto map = Value::create(Map{{key_a, a}, {key_b, b}});

        auto map_pred = make_builtin(
            [](builtin_args_t args) {
                return Value::create(args.at(1)->raw_get<Int>() == 2_f);
            },
            "pred");

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
            "op");

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
            "op");

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
            "op");

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

            CHECK_THROWS_MATCHES(fn->call({arr, fn_val}),
                                 Frost_Recoverable_Error,
                                 MessageMatches(ContainsSubstring("Array")));
        }

        {
            auto callable = mock::Mock_Callable::make();
            auto fn_val = Value::create(Function{callable});
            REQUIRE_CALL(*callable, call(_)).RETURN(Value::create(Map{}));

            CHECK_THROWS_MATCHES(fn->call({arr, fn_val}),
                                 Frost_Recoverable_Error,
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

            CHECK_THROWS_MATCHES(fn->call({arr, fn_val}),
                                 Frost_Recoverable_Error,
                                 MessageMatches(ContainsSubstring("Array")));
        }

        {
            auto callable = mock::Mock_Callable::make();
            auto fn_val = Value::create(Function{callable});
            REQUIRE_CALL(*callable, call(_)).RETURN(Value::create(Map{}));

            CHECK_THROWS_MATCHES(fn->call({arr, fn_val}),
                                 Frost_Recoverable_Error,
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
            .LR_WITH(_1.size() == 2 && require_int(_1[0]) == 3_f && _1[1] == c)
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

    SECTION("partition semantics")
    {
        auto fn = lookup(table, "partition");
        auto a = Value::create(0_f);
        auto b = Value::create(1_f);
        auto c = Value::create(2_f);
        auto d = Value::create(3_f);
        auto arr = Value::create(Array{a, b, c, d});

        auto callable = mock::Mock_Callable::make();
        auto fn_val = Value::create(Function{callable});

        trompeloeil::sequence seq;
        REQUIRE_CALL(*callable, call(_))
            .LR_WITH(_1.size() == 1 && _1[0] == a)
            .IN_SEQUENCE(seq)
            .RETURN(Value::create(true));
        REQUIRE_CALL(*callable, call(_))
            .LR_WITH(_1.size() == 1 && _1[0] == b)
            .IN_SEQUENCE(seq)
            .RETURN(Value::create(false));
        REQUIRE_CALL(*callable, call(_))
            .LR_WITH(_1.size() == 1 && _1[0] == c)
            .IN_SEQUENCE(seq)
            .RETURN(Value::create(true));
        REQUIRE_CALL(*callable, call(_))
            .LR_WITH(_1.size() == 1 && _1[0] == d)
            .IN_SEQUENCE(seq)
            .RETURN(Value::create(false));

        auto res = fn->call({arr, fn_val});
        REQUIRE(res->is<Map>());
        const auto& out = res->raw_get<Map>();
        REQUIRE(out.size() == 2);

        auto pass_key = Value::create("pass"s);
        auto fail_key = Value::create("fail"s);

        auto pass_it = out.find(pass_key);
        REQUIRE(pass_it != out.end());
        require_array_eq(pass_it->second, {a, c});

        auto fail_it = out.find(fail_key);
        REQUIRE(fail_it != out.end());
        require_array_eq(fail_it->second, {b, d});
    }

    SECTION("partition empty array returns empty buckets")
    {
        auto fn = lookup(table, "partition");
        auto empty = Value::create(Array{});
        auto callable = mock::Mock_Callable::make();
        auto fn_val = Value::create(Function{callable});

        FORBID_CALL(*callable, call(_));

        auto res = fn->call({empty, fn_val});
        REQUIRE(res->is<Map>());
        const auto& out = res->raw_get<Map>();
        REQUIRE(out.size() == 2);

        auto pass_key = Value::create("pass"s);
        auto fail_key = Value::create("fail"s);

        auto pass_it = out.find(pass_key);
        REQUIRE(pass_it != out.end());
        require_array_eq(pass_it->second, {});

        auto fail_it = out.find(fail_key);
        REQUIRE(fail_it != out.end());
        require_array_eq(fail_it->second, {});
    }

    SECTION("partition propagates predicate errors")
    {
        auto fn = lookup(table, "partition");
        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto arr = Value::create(Array{a, b});

        auto callable = mock::Mock_Callable::make();
        auto fn_val = Value::create(Function{callable});

        trompeloeil::sequence seq;
        REQUIRE_CALL(*callable, call(_))
            .LR_WITH(_1.size() == 1 && _1[0] == a)
            .IN_SEQUENCE(seq)
            .RETURN(Value::create(true));
        REQUIRE_CALL(*callable, call(_))
            .LR_WITH(_1.size() == 1 && _1[0] == b)
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

        auto all_falsy =
            Value::create(Array{Value::null(), Value::create(false)});
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
            "pred_lt_two");

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
            "count");

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
            [](builtin_args_t) {
                return Value::create(true);
            },
            "pred_true");
        auto pred_false = make_builtin(
            [](builtin_args_t) {
                return Value::create(false);
            },
            "pred_false");

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
            [](builtin_args_t) {
                return Value::null();
            },
            "pred_null");
        auto pred_int = make_builtin(
            [](builtin_args_t) {
                return Value::create(0_f);
            },
            "pred_int");

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
            "pred_any");

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
            "pred_all");

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
            "pred_none");

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
            "boom");

        CHECK_THROWS_WITH(any_fn->call({arr, Value::create(Function{boom})}),
                          ContainsSubstring("kaboom"));
        CHECK_THROWS_WITH(all_fn->call({arr, Value::create(Function{boom})}),
                          ContainsSubstring("kaboom"));
        CHECK_THROWS_WITH(none_fn->call({arr, Value::create(Function{boom})}),
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
            "desc");

        require_array_eq(fn->call({arr, Value::create(Function{desc})}),
                         {a, c, b});

        auto empty = Value::create(Array{});
        require_array_eq(fn->call({empty}), {});

        auto single = Value::create(Array{a});
        require_array_eq(fn->call({single}), {a});

        auto dup_arr =
            Value::create(Array{Value::create(2_f), Value::create(1_f),
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
            "truthy_int_cmp");

        require_array_eq(
            fn->call({arr, Value::create(Function{truthy_int_cmp})}),
            {b, c, a});
    }

    SECTION("sorted predicate error propagates")
    {
        auto fn = lookup(table, "sorted");
        auto arr = Value::create(Array{Value::create(1_f), Value::create(0_f)});
        auto boom = make_builtin(
            [](builtin_args_t) -> Value_Ptr {
                throw Frost_Recoverable_Error{"kaboom"};
            },
            "boom");

        CHECK_THROWS_WITH(fn->call({arr, Value::create(Function{boom})}),
                          ContainsSubstring("kaboom"));
    }

    SECTION("sorted rejects non-comparable elements")
    {
        auto fn = lookup(table, "sorted");
        auto mixed =
            Value::create(Array{Value::create(1_f), Value::create("a"s)});

        CHECK_THROWS_WITH(fn->call({mixed}),
                          ContainsSubstring("compare incompatible types"));
    }

    SECTION("sort_by semantics")
    {
        auto fn = lookup(table, "sort_by");

        // Sort by projection: sort integers by their negation (descending)
        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto c = Value::create(3_f);
        auto arr = Value::create(Array{b, c, a});

        auto negate = make_builtin(
            [](builtin_args_t args) {
                return args.at(0)->negate();
            },
            "negate");

        require_array_eq(fn->call({arr, Value::create(Function{negate})}),
                         {c, b, a});

        // Original array unchanged
        const auto& orig = arr->raw_get<Array>();
        REQUIRE(orig.size() == 3);
        CHECK(orig.at(0) == b);
        CHECK(orig.at(1) == c);
        CHECK(orig.at(2) == a);

        // Sort strings by length
        auto s1 = Value::create("bb"s);
        auto s2 = Value::create("a"s);
        auto s3 = Value::create("ccc"s);
        auto str_arr = Value::create(Array{s3, s1, s2});

        auto str_len = make_builtin(
            [](builtin_args_t args) {
                return Value::create(
                    static_cast<Int>(args.at(0)->raw_get<String>().size()));
            },
            "str_len");

        require_array_eq(fn->call({str_arr, Value::create(Function{str_len})}),
                         {s2, s1, s3});

        // Empty array
        require_array_eq(
            fn->call({Value::create(Array{}), Value::create(Function{negate})}),
            {});

        // Single element
        require_array_eq(fn->call({Value::create(Array{a}),
                                   Value::create(Function{negate})}),
                         {a});
    }

    SECTION("sort_by stability")
    {
        auto fn = lookup(table, "sort_by");

        // Elements with equal keys should preserve original order
        auto a = Value::create("ax"s);
        auto b = Value::create("bx"s);
        auto c = Value::create("cy"s);
        auto d = Value::create("dy"s);
        auto arr = Value::create(Array{c, a, d, b});

        // Project to second character -- a and b both have "x", c and d have
        // "y"
        auto second_char = make_builtin(
            [](builtin_args_t args) {
                const auto& s = args.at(0)->raw_get<String>();
                return Value::create(std::string(1, s.at(1)));
            },
            "second_char");

        auto result = fn->call({arr, Value::create(Function{second_char})});
        REQUIRE(result->is<Array>());
        const auto& out = result->raw_get<Array>();
        REQUIRE(out.size() == 4);
        // "x" keys first (c, d original order preserved), then "y" keys (a, b)
        CHECK(out.at(0) == a);
        CHECK(out.at(1) == b);
        CHECK(out.at(2) == c);
        CHECK(out.at(3) == d);
    }

    SECTION("sort_by projection called once per element")
    {
        auto fn = lookup(table, "sort_by");

        auto arr = Value::create(
            Array{Value::create(3_f), Value::create(1_f), Value::create(2_f)});

        int call_count = 0;
        auto counting_proj = make_builtin(
            [&](builtin_args_t args) {
                ++call_count;
                return args.at(0);
            },
            "counting_proj");

        fn->call({arr, Value::create(Function{counting_proj})});
        CHECK(call_count == 3);
    }

    SECTION("sort_by projection error propagates")
    {
        auto fn = lookup(table, "sort_by");
        auto arr = Value::create(Array{Value::create(1_f), Value::create(0_f)});
        auto boom = make_builtin(
            [](builtin_args_t) -> Value_Ptr {
                throw Frost_Recoverable_Error{"kaboom"};
            },
            "boom");

        CHECK_THROWS_WITH(fn->call({arr, Value::create(Function{boom})}),
                          ContainsSubstring("kaboom"));
    }

    SECTION("sort_by rejects non-comparable keys")
    {
        auto fn = lookup(table, "sort_by");

        // Projection returns mixed types that can't be compared
        int call_count = 0;
        auto mixed_proj = make_builtin(
            [&](builtin_args_t) -> Value_Ptr {
                if (call_count++ == 0)
                    return Value::create(1_f);
                return Value::create("a"s);
            },
            "mixed_proj");

        auto arr = Value::create(Array{Value::create(1_f), Value::create(2_f)});

        CHECK_THROWS_WITH(fn->call({arr, Value::create(Function{mixed_proj})}),
                          ContainsSubstring("compare incompatible types"));
    }

    SECTION("find semantics")
    {
        auto fn = lookup(table, "find");
        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto c = Value::create(3_f);
        auto arr = Value::create(Array{a, b, c});

        auto gt1 = make_builtin(
            [](builtin_args_t args) {
                return Value::greater_than(args.at(0), Value::create(1_f));
            },
            "gt1");

        // Returns the first matching element
        auto result = fn->call({arr, Value::create(Function{gt1})});
        CHECK(result == b);

        // Returns null when no element matches
        auto gt10 = make_builtin(
            [](builtin_args_t args) {
                return Value::greater_than(args.at(0), Value::create(10_f));
            },
            "gt10");
        CHECK(fn->call({arr, Value::create(Function{gt10})})->is<Null>());

        // Empty array returns null
        auto always = make_builtin(
            [](builtin_args_t) {
                return Value::create(true);
            },
            "always");
        CHECK(
            fn->call({Value::create(Array{}), Value::create(Function{always})})
                ->is<Null>());

        // Returns the exact same pointer, not a copy
        auto is2 = make_builtin(
            [&](builtin_args_t args) {
                return Value::equal(args.at(0), b);
            },
            "is2");
        CHECK(fn->call({arr, Value::create(Function{is2})}).get() == b.get());
    }

    SECTION("find short-circuits")
    {
        auto fn = lookup(table, "find");

        int call_count = 0;
        auto counting = make_builtin(
            [&](builtin_args_t args) {
                ++call_count;
                return Value::equal(args.at(0), Value::create(2_f));
            },
            "counting");

        auto arr = Value::create(
            Array{Value::create(1_f), Value::create(2_f), Value::create(3_f)});

        fn->call({arr, Value::create(Function{counting})});
        CHECK(call_count == 2);
    }

    SECTION("find predicate error propagates")
    {
        auto fn = lookup(table, "find");
        auto arr = Value::create(Array{Value::create(1_f)});
        auto boom = make_builtin(
            [](builtin_args_t) -> Value_Ptr {
                throw Frost_Recoverable_Error{"kaboom"};
            },
            "boom");

        CHECK_THROWS_WITH(fn->call({arr, Value::create(Function{boom})}),
                          ContainsSubstring("kaboom"));
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

TEST_CASE("Builtin repeat")
{
    Symbol_Table table;
    inject_builtins(table);
    auto fn = lookup(table, "repeat");

    SECTION("Injected")
    {
        CHECK(table.lookup("repeat")->is<Function>());
    }

    SECTION("Arity")
    {
        CHECK_THROWS_MATCHES(
            fn->call({}), Frost_User_Error,
            MessageMatches(ContainsSubstring("insufficient arguments")));
        CHECK_THROWS_MATCHES(
            fn->call({Value::create(1)}), Frost_User_Error,
            MessageMatches(ContainsSubstring("insufficient arguments")));
        CHECK_THROWS_MATCHES(
            fn->call({Value::create(1), Value::create(2), Value::create(3)}),
            Frost_User_Error,
            MessageMatches(ContainsSubstring("too many arguments")));
    }

    SECTION("Type error: second arg must be Int")
    {
        CHECK_THROWS_MATCHES(fn->call({Value::create(1), Value::create(2.5)}),
                             Frost_User_Error,
                             MessageMatches(ContainsSubstring("repeat")
                                            && ContainsSubstring("Int")));
    }

    SECTION("Negative count throws")
    {
        CHECK_THROWS_MATCHES(fn->call({Value::create(1), Value::create(-1)}),
                             Frost_User_Error,
                             MessageMatches(ContainsSubstring("repeat")
                                            && ContainsSubstring(">=0")));
    }

    SECTION("Count 0 returns empty array")
    {
        auto r = fn->call({Value::create(42), Value::create(0)});
        REQUIRE(r->is<Array>());
        CHECK(r->raw_get<Array>().empty());
    }

    SECTION("Count 1 returns single-element array")
    {
        auto val = Value::create(42);
        auto r = fn->call({val, Value::create(1)});
        REQUIRE(r->is<Array>());
        REQUIRE(r->raw_get<Array>().size() == 1);
        CHECK(r->raw_get<Array>()[0]->raw_get<Int>() == 42);
    }

    SECTION("Count N produces N copies")
    {
        auto val = Value::create(7);
        auto r = fn->call({val, Value::create(5)});
        REQUIRE(r->is<Array>());
        REQUIRE(r->raw_get<Array>().size() == 5);
        for (const auto& elem : r->raw_get<Array>())
            CHECK(elem->raw_get<Int>() == 7);
    }

    SECTION("Works with any value type")
    {
        // String
        {
            auto r = fn->call({Value::create("hi"s), Value::create(3)});
            REQUIRE(r->raw_get<Array>().size() == 3);
            for (const auto& e : r->raw_get<Array>())
                CHECK(e->raw_get<String>() == "hi");
        }
        // Null
        {
            auto r = fn->call({Value::null(), Value::create(2)});
            REQUIRE(r->raw_get<Array>().size() == 2);
            for (const auto& e : r->raw_get<Array>())
                CHECK(e->is<Null>());
        }
        // Array
        {
            auto inner =
                Value::create(Array{Value::create(1), Value::create(2)});
            auto r = fn->call({inner, Value::create(2)});
            REQUIRE(r->raw_get<Array>().size() == 2);
            for (const auto& e : r->raw_get<Array>())
                CHECK(e->raw_get<Array>().size() == 2);
        }
    }
}

TEST_CASE("Builtin flatten")
{
    Symbol_Table table;
    inject_builtins(table);
    auto fn = lookup(table, "flatten");

    auto i = [](Int v) {
        return Value::create(v);
    };
    auto arr = [](auto&&... elems) {
        return Value::create(Array{elems...});
    };

    // Compare by value, not pointer identity
    auto ints_eq = [](const Value_Ptr& result, std::vector<Int> expected) {
        REQUIRE(result->is<Array>());
        const auto& a = result->raw_get<Array>();
        REQUIRE(a.size() == expected.size());
        for (std::size_t k = 0; k < expected.size(); ++k)
        {
            REQUIRE(a[k]->is<Int>());
            CHECK(a[k]->raw_get<Int>() == expected[k]);
        }
    };

    SECTION("Injected")
    {
        CHECK(table.lookup("flatten")->is<Function>());
    }

    SECTION("Arity")
    {
        CHECK_THROWS_MATCHES(
            fn->call({}), Frost_User_Error,
            MessageMatches(ContainsSubstring("insufficient arguments")));
        CHECK_THROWS_MATCHES(
            fn->call({Value::create(Array{}), i(1), i(2)}), Frost_User_Error,
            MessageMatches(ContainsSubstring("too many arguments")));
    }

    SECTION("Type errors")
    {
        CHECK_THROWS_MATCHES(fn->call({Value::create(42)}), Frost_User_Error,
                             MessageMatches(ContainsSubstring("flatten")
                                            && ContainsSubstring("Array")));
        CHECK_THROWS_MATCHES(
            fn->call({Value::create(Array{}), Value::create(2.5)}),
            Frost_User_Error,
            MessageMatches(ContainsSubstring("flatten")
                           && ContainsSubstring("Int")
                           && ContainsSubstring("(n)")));
    }

    SECTION("Negative n throws")
    {
        CHECK_THROWS_MATCHES(fn->call({Value::create(Array{}), i(-1)}),
                             Frost_User_Error,
                             MessageMatches(ContainsSubstring("flatten")
                                            && ContainsSubstring(">=0")));
    }

    SECTION("Empty array returns empty array")
    {
        auto r = fn->call({Value::create(Array{})});
        REQUIRE(r->is<Array>());
        CHECK(r->raw_get<Array>().empty());
    }

    SECTION("Flat array is unchanged")
    {
        ints_eq(fn->call({arr(i(1), i(2), i(3))}), {1, 2, 3});
    }

    SECTION("Non-array elements are left untouched")
    {
        auto r = fn->call({arr(i(1), Value::null(), Value::create("x"s))});
        REQUIRE(r->is<Array>());
        const auto& a = r->raw_get<Array>();
        REQUIRE(a.size() == 3);
        CHECK(a[0]->raw_get<Int>() == 1);
        CHECK(a[1]->is<Null>());
        CHECK(a[2]->raw_get<String>() == "x");
    }

    SECTION("flatten (recursive)")
    {
        SECTION("One level")
        {
            ints_eq(fn->call({arr(i(1), arr(i(2), i(3)), i(4))}), {1, 2, 3, 4});
        }

        SECTION("Deeply nested")
        {
            ints_eq(fn->call({arr(i(1), arr(i(2), arr(i(3), arr(i(4)))))}),
                    {1, 2, 3, 4});
        }

        SECTION("Mixed depth nesting")
        {
            ints_eq(fn->call({arr(arr(i(1), i(2)), i(3), arr(arr(i(4))))}),
                    {1, 2, 3, 4});
        }

        SECTION("Empty nested arrays are dropped")
        {
            ints_eq(fn->call({arr(arr(), i(1), arr(), i(2))}), {1, 2});
        }
    }

    SECTION("flatten(arr, n)")
    {
        // [1, [2, [3, [4]]]]
        auto nested = arr(i(1), arr(i(2), arr(i(3), arr(i(4)))));

        SECTION("n=0 returns array unchanged")
        {
            auto r = fn->call({nested, i(0)});
            REQUIRE(r->is<Array>());
            const auto& a = r->raw_get<Array>();
            REQUIRE(a.size() == 2);
            CHECK(a[0]->raw_get<Int>() == 1);
            CHECK(a[1]->is<Array>());
        }

        SECTION("n=1 flattens one level")
        {
            // [1, [2, [3, [4]]]] -> [1, 2, [3, [4]]]
            auto r = fn->call({nested, i(1)});
            REQUIRE(r->is<Array>());
            const auto& a = r->raw_get<Array>();
            REQUIRE(a.size() == 3);
            CHECK(a[0]->raw_get<Int>() == 1);
            CHECK(a[1]->raw_get<Int>() == 2);
            CHECK(a[2]->is<Array>());
        }

        SECTION("n=2 flattens two levels")
        {
            // [1, [2, [3, [4]]]] -> [1, 2, 3, [4]]
            auto r = fn->call({nested, i(2)});
            REQUIRE(r->is<Array>());
            const auto& a = r->raw_get<Array>();
            REQUIRE(a.size() == 4);
            CHECK(a[0]->raw_get<Int>() == 1);
            CHECK(a[1]->raw_get<Int>() == 2);
            CHECK(a[2]->raw_get<Int>() == 3);
            CHECK(a[3]->is<Array>());
        }

        SECTION("Large n fully flattens")
        {
            ints_eq(fn->call({nested, i(100)}), {1, 2, 3, 4});
        }

        SECTION("n > 0 on flat array is a no-op")
        {
            ints_eq(fn->call({arr(i(1), i(2), i(3)), i(5)}), {1, 2, 3});
        }

        SECTION("Empty array with n returns empty array")
        {
            auto r = fn->call({Value::create(Array{}), i(1)});
            REQUIRE(r->is<Array>());
            CHECK(r->raw_get<Array>().empty());
        }
    }

    SECTION("zip_with")
    {
        auto fn = lookup(table, "zip_with");

        auto add_fn = Value::create(Function{std::make_shared<Builtin>(
            [](builtin_args_t a) {
                return Value::create(a.at(0)->raw_get<Int>()
                                    + a.at(1)->raw_get<Int>());
            },
            "add")});

        auto mul_fn = Value::create(Function{std::make_shared<Builtin>(
            [](builtin_args_t a) {
                return Value::create(a.at(0)->raw_get<Int>()
                                    * a.at(1)->raw_get<Int>());
            },
            "mul")});

        auto add3_fn = Value::create(Function{std::make_shared<Builtin>(
            [](builtin_args_t a) {
                return Value::create(a.at(0)->raw_get<Int>()
                                    + a.at(1)->raw_get<Int>()
                                    + a.at(2)->raw_get<Int>());
            },
            "add3")});

        SECTION("Arity: too few arguments")
        {
            CHECK_THROWS_WITH(fn->call({}),
                              ContainsSubstring("insufficient"));
            CHECK_THROWS_WITH(fn->call({add_fn}),
                              ContainsSubstring("insufficient"));
            CHECK_THROWS_WITH(
                fn->call({add_fn, Value::create(Array{})}),
                ContainsSubstring("insufficient"));
        }

        SECTION("Type: first arg must be Function")
        {
            CHECK_THROWS_WITH(
                fn->call({Value::create(42_f),
                          Value::create(Array{}),
                          Value::create(Array{})}),
                ContainsSubstring("Function"));
        }

        SECTION("Type: remaining args must be Array")
        {
            CHECK_THROWS_WITH(
                fn->call({add_fn,
                          Value::create(Array{}),
                          Value::create(42_f)}),
                ContainsSubstring("Array"));
        }

        SECTION("Two equal-length arrays")
        {
            auto r = fn->call({
                add_fn,
                Value::create(Array{Value::create(1_f), Value::create(2_f)}),
                Value::create(Array{Value::create(10_f), Value::create(20_f)}),
            });
            REQUIRE(r->is<Array>());
            const auto& arr = r->raw_get<Array>();
            REQUIRE(arr.size() == 2);
            CHECK(arr[0]->raw_get<Int>() == 11_f);
            CHECK(arr[1]->raw_get<Int>() == 22_f);
        }

        SECTION("Truncates to shortest")
        {
            auto r = fn->call({
                add_fn,
                Value::create(Array{Value::create(1_f), Value::create(2_f),
                                    Value::create(3_f)}),
                Value::create(Array{Value::create(10_f), Value::create(20_f)}),
            });
            REQUIRE(r->is<Array>());
            const auto& arr = r->raw_get<Array>();
            REQUIRE(arr.size() == 2);
            CHECK(arr[0]->raw_get<Int>() == 11_f);
            CHECK(arr[1]->raw_get<Int>() == 22_f);
        }

        SECTION("Three arrays")
        {
            auto r = fn->call({
                add3_fn,
                Value::create(Array{Value::create(1_f), Value::create(2_f)}),
                Value::create(Array{Value::create(10_f), Value::create(20_f)}),
                Value::create(Array{Value::create(100_f), Value::create(200_f)}),
            });
            REQUIRE(r->is<Array>());
            const auto& arr = r->raw_get<Array>();
            REQUIRE(arr.size() == 2);
            CHECK(arr[0]->raw_get<Int>() == 111_f);
            CHECK(arr[1]->raw_get<Int>() == 222_f);
        }

        SECTION("Empty arrays produce empty result")
        {
            auto r = fn->call({
                add_fn,
                Value::create(Array{}),
                Value::create(Array{}),
            });
            REQUIRE(r->is<Array>());
            CHECK(r->raw_get<Array>().empty());
        }

        SECTION("One empty array truncates to empty")
        {
            auto r = fn->call({
                add_fn,
                Value::create(Array{Value::create(1_f)}),
                Value::create(Array{}),
            });
            REQUIRE(r->is<Array>());
            CHECK(r->raw_get<Array>().empty());
        }

        SECTION("Different function: multiply")
        {
            auto r = fn->call({
                mul_fn,
                Value::create(Array{Value::create(3_f), Value::create(5_f)}),
                Value::create(Array{Value::create(7_f), Value::create(11_f)}),
            });
            REQUIRE(r->is<Array>());
            const auto& arr = r->raw_get<Array>();
            REQUIRE(arr.size() == 2);
            CHECK(arr[0]->raw_get<Int>() == 21_f);
            CHECK(arr[1]->raw_get<Int>() == 55_f);
        }

        SECTION("Equivalent to map zip(...) with f")
        {
            // zip_with(f, a, b) should equal map zip(a, b) with spread(f)
            auto a = Value::create(Array{Value::create(1_f), Value::create(2_f)});
            auto b = Value::create(Array{Value::create(10_f), Value::create(20_f)});

            auto zw_result = fn->call({add_fn, a, b});

            auto zip_fn = lookup(table, "zip");
            auto zipped = zip_fn->call({a, b});
            // Manually apply add to each pair
            const auto& pairs = zipped->raw_get<Array>();
            Array manual;
            for (const auto& pair : pairs)
            {
                const auto& row = pair->raw_get<Array>();
                manual.push_back(
                    Value::create(row[0]->raw_get<Int>()
                                  + row[1]->raw_get<Int>()));
            }

            REQUIRE(zw_result->is<Array>());
            const auto& zw = zw_result->raw_get<Array>();
            REQUIRE(zw.size() == manual.size());
            for (std::size_t j = 0; j < zw.size(); ++j)
                CHECK(Value::equal(zw[j], manual[j])->truthy());
        }
    }

    SECTION("xprod_with")
    {
        auto fn = lookup(table, "xprod_with");

        auto add_fn = Value::create(Function{std::make_shared<Builtin>(
            [](builtin_args_t a) {
                return Value::create(a.at(0)->raw_get<Int>()
                                    + a.at(1)->raw_get<Int>());
            },
            "add")});

        auto concat_fn = Value::create(Function{std::make_shared<Builtin>(
            [](builtin_args_t a) {
                return Value::create(a.at(0)->raw_get<String>()
                                    + a.at(1)->raw_get<String>());
            },
            "concat")});

        auto add3_fn = Value::create(Function{std::make_shared<Builtin>(
            [](builtin_args_t a) {
                return Value::create(a.at(0)->raw_get<Int>()
                                    + a.at(1)->raw_get<Int>()
                                    + a.at(2)->raw_get<Int>());
            },
            "add3")});

        SECTION("Arity: too few arguments")
        {
            CHECK_THROWS_WITH(fn->call({}),
                              ContainsSubstring("insufficient"));
            CHECK_THROWS_WITH(fn->call({add_fn}),
                              ContainsSubstring("insufficient"));
            CHECK_THROWS_WITH(
                fn->call({add_fn, Value::create(Array{})}),
                ContainsSubstring("insufficient"));
        }

        SECTION("Type: first arg must be Function")
        {
            CHECK_THROWS_WITH(
                fn->call({Value::create(42_f),
                          Value::create(Array{}),
                          Value::create(Array{})}),
                ContainsSubstring("Function"));
        }

        SECTION("Type: remaining args must be Array")
        {
            CHECK_THROWS_WITH(
                fn->call({add_fn,
                          Value::create(Array{}),
                          Value::create(42_f)}),
                ContainsSubstring("Array"));
        }

        SECTION("Two arrays: applies f to each pair in the product")
        {
            auto r = fn->call({
                add_fn,
                Value::create(Array{Value::create(10_f), Value::create(20_f)}),
                Value::create(Array{Value::create(1_f), Value::create(2_f)}),
            });
            REQUIRE(r->is<Array>());
            const auto& arr = r->raw_get<Array>();
            // 2x2 = 4 combinations: 10+1, 10+2, 20+1, 20+2
            REQUIRE(arr.size() == 4);
            CHECK(arr[0]->raw_get<Int>() == 11_f);
            CHECK(arr[1]->raw_get<Int>() == 12_f);
            CHECK(arr[2]->raw_get<Int>() == 21_f);
            CHECK(arr[3]->raw_get<Int>() == 22_f);
        }

        SECTION("Three arrays")
        {
            auto r = fn->call({
                add3_fn,
                Value::create(Array{Value::create(100_f), Value::create(200_f)}),
                Value::create(Array{Value::create(10_f)}),
                Value::create(Array{Value::create(1_f), Value::create(2_f)}),
            });
            REQUIRE(r->is<Array>());
            const auto& arr = r->raw_get<Array>();
            // 2x1x2 = 4 combinations
            REQUIRE(arr.size() == 4);
            CHECK(arr[0]->raw_get<Int>() == 111_f);
            CHECK(arr[1]->raw_get<Int>() == 112_f);
            CHECK(arr[2]->raw_get<Int>() == 211_f);
            CHECK(arr[3]->raw_get<Int>() == 212_f);
        }

        SECTION("Empty array produces empty result")
        {
            auto r = fn->call({
                add_fn,
                Value::create(Array{Value::create(1_f)}),
                Value::create(Array{}),
            });
            REQUIRE(r->is<Array>());
            CHECK(r->raw_get<Array>().empty());
        }

        SECTION("String concatenation across product")
        {
            auto r = fn->call({
                concat_fn,
                Value::create(Array{Value::create("a"s), Value::create("b"s)}),
                Value::create(Array{Value::create("1"s), Value::create("2"s)}),
            });
            REQUIRE(r->is<Array>());
            const auto& arr = r->raw_get<Array>();
            REQUIRE(arr.size() == 4);
            CHECK(arr[0]->raw_get<String>() == "a1");
            CHECK(arr[1]->raw_get<String>() == "a2");
            CHECK(arr[2]->raw_get<String>() == "b1");
            CHECK(arr[3]->raw_get<String>() == "b2");
        }

        SECTION("Equivalent to map xprod(...) with spread(f)")
        {
            auto a = Value::create(Array{Value::create(1_f), Value::create(2_f)});
            auto b = Value::create(Array{Value::create(10_f), Value::create(20_f)});

            auto xw_result = fn->call({add_fn, a, b});

            auto xprod_fn = lookup(table, "xprod");
            auto product = xprod_fn->call({a, b});
            const auto& tuples = product->raw_get<Array>();
            Array manual;
            for (const auto& tup : tuples)
            {
                const auto& row = tup->raw_get<Array>();
                manual.push_back(
                    Value::create(row[0]->raw_get<Int>()
                                  + row[1]->raw_get<Int>()));
            }

            REQUIRE(xw_result->is<Array>());
            const auto& xw = xw_result->raw_get<Array>();
            REQUIRE(xw.size() == manual.size());
            for (std::size_t j = 0; j < xw.size(); ++j)
                CHECK(Value::equal(xw[j], manual[j])->truthy());
        }
    }
}
