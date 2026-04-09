#include <catch2/catch_all.hpp>
#include <catch2/trompeloeil.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/mock/mock-callable.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <frost/builtin.hpp>
#include <frost/builtins-common.hpp>

using namespace frst;

using namespace std::literals;

using namespace Catch::Matchers;
using trompeloeil::_;

namespace
{
using Call_List = std::vector<std::vector<Value_Ptr>>;

void record_call(Call_List& calls, std::span<const Value_Ptr> args)
{
    calls.emplace_back(args.begin(), args.end());
}
} // namespace

TEST_CASE("Builtin try_call")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    Symbol_Table table;
    inject_builtins(table);
    auto try_call_val = table.lookup("try_call");
    REQUIRE(try_call_val->is<Function>());
    auto try_call = try_call_val->get<Function>().value();

    auto key_ok = Value::create("ok"s);
    auto key_value = Value::create("value"s);
    auto key_error = Value::create("error"s);

    SECTION("Injected")
    {
        CHECK(try_call_val->is<Function>());
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(try_call->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(try_call->call({}),
                          ContainsSubstring("Called with 0"));
        CHECK_THROWS_WITH(try_call->call({}),
                          ContainsSubstring("requires at least 1"));

        CHECK_THROWS_WITH(
            try_call->call({Value::null(), Value::null(), Value::null()}),
            ContainsSubstring("too many arguments"));
        CHECK_THROWS_WITH(
            try_call->call({Value::null(), Value::null(), Value::null()}),
            ContainsSubstring("Called with 3"));
        CHECK_THROWS_WITH(
            try_call->call({Value::null(), Value::null(), Value::null()}),
            ContainsSubstring("no more than 2"));
    }

    SECTION("Omitted args calls zero-arg function")
    {
        auto callable = mock::Mock_Callable::make();
        auto fn = Value::create(Function{callable});

        REQUIRE_CALL(*callable, call(_))
            .WITH(_1.empty())
            .RETURN(Value::create(42_f));

        auto res = try_call->call({fn});
        REQUIRE(res->is<Map>());
        auto map = res->get<Map>().value();
        CHECK(map.at(key_ok)->get<Bool>().value() == true);
        CHECK(map.at(key_value)->get<Int>().value() == 42_f);
    }

    SECTION("Omitted args captures error from zero-arg function")
    {
        auto callable = mock::Mock_Callable::make();
        auto fn = Value::create(Function{callable});

        REQUIRE_CALL(*callable, call(_))
            .WITH(_1.empty())
            .THROW(Frost_Recoverable_Error{"no args boom"});

        auto res = try_call->call({fn});
        REQUIRE(res->is<Map>());
        auto map = res->get<Map>().value();
        CHECK(map.at(key_ok)->get<Bool>().value() == false);
        CHECK_THAT(map.at(key_error)->raw_get<String>(),
                   ContainsSubstring("no args boom"));
    }

    SECTION("Type checks")
    {
        SECTION("First argument must be Function")
        {
            auto bad_fn = Value::create(3.14);
            auto args = Value::create(Array{});
            CHECK_THROWS_WITH(try_call->call({bad_fn, args}),
                              ContainsSubstring("Function try_call requires"));
            CHECK_THROWS_WITH(try_call->call({bad_fn, args}),
                              ContainsSubstring("function"));
            CHECK_THROWS_WITH(try_call->call({bad_fn, args}),
                              EndsWith(std::string{bad_fn->type_name()}));
        }

        SECTION("Second argument must be Array")
        {
            auto fn = Value::create(Function{mock::Mock_Callable::make()});
            auto bad_args = Value::create("nope"s);
            CHECK_THROWS_WITH(try_call->call({fn, bad_args}),
                              ContainsSubstring("Function try_call requires"));
            CHECK_THROWS_WITH(try_call->call({fn, bad_args}),
                              ContainsSubstring("args"));
            CHECK_THROWS_WITH(try_call->call({fn, bad_args}),
                              EndsWith(std::string{bad_args->type_name()}));
        }
    }

    SECTION("Success returns ok=true and value")
    {
        auto callable = mock::Mock_Callable::make();
        auto ret = Value::create(42_f);
        auto fn = Value::create(Function{callable});
        auto args = Value::create(Array{});
        Call_List calls;

        REQUIRE_CALL(*callable, call(_))
            .LR_SIDE_EFFECT(record_call(calls, _1))
            .RETURN(ret);

        auto res = try_call->call({fn, args});
        REQUIRE(res->is<Map>());
        auto map = res->get<Map>().value();
        CHECK(map.size() == 2);
        REQUIRE(map.contains(key_ok));
        REQUIRE(map.contains(key_value));
        CHECK_FALSE(map.contains(key_error));

        CHECK(map.at(key_ok)->get<Bool>().value() == true);
        CHECK(map.at(key_value) == ret);
        CHECK(calls.size() == 1);
        CHECK(calls.at(0).empty());
    }

    SECTION("Success preserves Null return as value")
    {
        auto callable = mock::Mock_Callable::make();
        auto fn = Value::create(Function{callable});
        auto args = Value::create(Array{});
        Call_List calls;

        REQUIRE_CALL(*callable, call(_))
            .LR_SIDE_EFFECT(record_call(calls, _1))
            .RETURN(Value::null());

        auto res = try_call->call({fn, args});
        REQUIRE(res->is<Map>());
        auto map = res->get<Map>().value();
        CHECK(map.size() == 2);
        REQUIRE(map.contains(key_ok));
        REQUIRE(map.contains(key_value));
        CHECK(map.at(key_ok)->get<Bool>().value() == true);
        CHECK(map.at(key_value)->is<Null>());
    }

    SECTION("Calls function with array elements (call semantics)")
    {
        auto callable = mock::Mock_Callable::make();
        auto fn = Value::create(Function{callable});
        auto a = Value::create(1_f);
        auto b = Value::create("hi"s);
        auto args = Value::create(Array{a, b});
        Call_List calls;

        REQUIRE_CALL(*callable, call(_))
            .LR_SIDE_EFFECT(record_call(calls, _1))
            .LR_WITH(_1.size() == 2 && _1[0] == a && _1[1] == b)
            .RETURN(Value::null());

        auto res = try_call->call({fn, args});
        REQUIRE(res->is<Map>());
        auto map = res->get<Map>().value();
        CHECK(map.size() == 2);
        REQUIRE(map.contains(key_ok));
        REQUIRE(map.contains(key_value));
        REQUIRE(calls.size() == 1);
        REQUIRE(calls.at(0).size() == 2);
        CHECK(calls.at(0).at(0) == a);
        CHECK(calls.at(0).at(1) == b);
    }

    SECTION("User error returns ok=false and error message")
    {
        auto callable = mock::Mock_Callable::make();
        auto fn = Value::create(Function{callable});
        auto args = Value::create(Array{});

        REQUIRE_CALL(*callable, call(_)).THROW(Frost_Recoverable_Error{"boom"});

        auto res = try_call->call({fn, args});
        REQUIRE(res->is<Map>());
        auto map = res->get<Map>().value();
        CHECK(map.size() == 2);
        REQUIRE(map.contains(key_ok));
        REQUIRE(map.contains(key_error));
        CHECK_FALSE(map.contains(key_value));

        CHECK(map.at(key_ok)->get<Bool>().value() == false);
        CHECK(map.at(key_error)->get<String>().value() == "boom");
    }

    SECTION("Internal error propagates")
    {
        auto callable = mock::Mock_Callable::make();
        auto fn = Value::create(Function{callable});
        auto args = Value::create(Array{});

        REQUIRE_CALL(*callable, call(_))
            .THROW(Frost_Interpreter_Error{"internal boom"});
        CHECK_THROWS_WITH(try_call->call({fn, args}),
                          ContainsSubstring("internal boom"));
    }
}

TEST_CASE("Builtin error")
{
    Symbol_Table table;
    inject_builtins(table);
    auto error_fn = table.lookup("error")->get<Function>().value();

    SECTION("Throws recoverable error with message")
    {
        CHECK_THROWS_AS(error_fn->call({Value::create("boom"s)}),
                        Frost_Recoverable_Error);
        CHECK_THROWS_WITH(error_fn->call({Value::create("boom"s)}), "boom");
    }

    SECTION("Catchable by try_call")
    {
        auto try_call_fn = table.lookup("try_call")->get<Function>().value();
        auto thrower =
            Value::create(system_function([&](builtin_args_t) -> Value_Ptr {
                error_fn->call({Value::create("caught"s)});
                return Value::null();
            }));
        auto result = try_call_fn->call({thrower});
        REQUIRE(result->is<Map>());
        auto map = result->get<Map>().value();
        CHECK(map.at(Value::create("ok"s))->get<Bool>().value() == false);
        CHECK_THAT(map.at(Value::create("error"s))->raw_get<String>(),
                   ContainsSubstring("caught"));
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(error_fn->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(
            error_fn->call({Value::create("a"s), Value::create("b"s)}),
            ContainsSubstring("too many arguments"));
    }

    SECTION("Type constraint")
    {
        CHECK_THROWS_WITH(error_fn->call({Value::create(42_f)}),
                          ContainsSubstring("String"));
    }
}

TEST_CASE("Builtin fatal")
{
    Symbol_Table table;
    inject_builtins(table);
    auto fatal_fn = table.lookup("fatal")->get<Function>().value();

    SECTION("Throws unrecoverable error with message")
    {
        CHECK_THROWS_AS(fatal_fn->call({Value::create("kaboom"s)}),
                        Frost_Unrecoverable_Error);
        CHECK_THROWS_WITH(fatal_fn->call({Value::create("kaboom"s)}), "kaboom");
    }

    SECTION("Not catchable by try_call")
    {
        auto try_call_fn = table.lookup("try_call")->get<Function>().value();
        auto thrower =
            Value::create(system_function([&](builtin_args_t) -> Value_Ptr {
                fatal_fn->call({Value::create("uncatchable"s)});
                return Value::null();
            }));
        CHECK_THROWS_AS(try_call_fn->call({thrower}),
                        Frost_Unrecoverable_Error);
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(fatal_fn->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(
            fatal_fn->call({Value::create("a"s), Value::create("b"s)}),
            ContainsSubstring("too many arguments"));
    }

    SECTION("Type constraint")
    {
        CHECK_THROWS_WITH(fatal_fn->call({Value::create(42_f)}),
                          ContainsSubstring("String"));
    }
}
