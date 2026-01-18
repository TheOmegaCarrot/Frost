#include <catch2/catch_all.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <frost/builtin.hpp>

using namespace frst;

using namespace std::literals;

using namespace Catch::Matchers;

namespace
{
struct Recording_Callable final : Callable
{
    mutable std::vector<std::vector<Value_Ptr>> calls;
    Value_Ptr result;

    Value_Ptr call(std::span<const Value_Ptr> args) const override
    {
        calls.emplace_back(args.begin(), args.end());
        return result ? result : Value::null();
    }

    std::string debug_dump() const override
    {
        return "<recording>";
    }
};

struct Throwing_User_Callable final : Callable
{
    explicit Throwing_User_Callable(std::string msg)
        : msg{std::move(msg)}
    {
    }

    Value_Ptr call(std::span<const Value_Ptr>) const override
    {
        throw Frost_User_Error{msg};
    }

    std::string debug_dump() const override
    {
        return "<throw-user>";
    }

    std::string msg;
};

struct Throwing_Internal_Callable final : Callable
{
    Value_Ptr call(std::span<const Value_Ptr>) const override
    {
        throw Frost_Internal_Error{"internal boom"};
    }

    std::string debug_dump() const override
    {
        return "<throw-internal>";
    }
};
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
                          ContainsSubstring("requires at least 2"));

        CHECK_THROWS_WITH(try_call->call({Value::null(), Value::null(),
                                          Value::null()}),
                          ContainsSubstring("too many arguments"));
        CHECK_THROWS_WITH(try_call->call({Value::null(), Value::null(),
                                          Value::null()}),
                          ContainsSubstring("Called with 3"));
        CHECK_THROWS_WITH(try_call->call({Value::null(), Value::null(),
                                          Value::null()}),
                          ContainsSubstring("no more than 2"));
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
            auto fn = Value::create(
                Function{std::make_shared<Recording_Callable>()});
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
        auto callable = std::make_shared<Recording_Callable>();
        auto ret = Value::create(42_f);
        callable->result = ret;
        auto fn = Value::create(Function{callable});
        auto args = Value::create(Array{});

        auto res = try_call->call({fn, args});
        REQUIRE(res->is<Map>());
        auto map = res->get<Map>().value();
        CHECK(map.size() == 2);
        REQUIRE(map.contains(key_ok));
        REQUIRE(map.contains(key_value));
        CHECK_FALSE(map.contains(key_error));

        CHECK(map.at(key_ok)->get<Bool>().value() == true);
        CHECK(map.at(key_value) == ret);
        CHECK(callable->calls.size() == 1);
        CHECK(callable->calls.at(0).empty());
    }

    SECTION("Success preserves Null return as value")
    {
        auto callable = std::make_shared<Recording_Callable>();
        callable->result = Value::null();
        auto fn = Value::create(Function{callable});
        auto args = Value::create(Array{});

        auto res = try_call->call({fn, args});
        REQUIRE(res->is<Map>());
        auto map = res->get<Map>().value();
        CHECK(map.size() == 2);
        REQUIRE(map.contains(key_ok));
        REQUIRE(map.contains(key_value));
        CHECK(map.at(key_ok)->get<Bool>().value() == true);
        CHECK(map.at(key_value)->is<Null>());
    }

    SECTION("Calls function with array elements (pack_call semantics)")
    {
        auto callable = std::make_shared<Recording_Callable>();
        auto fn = Value::create(Function{callable});
        auto a = Value::create(1_f);
        auto b = Value::create("hi"s);
        auto args = Value::create(Array{a, b});

        auto res = try_call->call({fn, args});
        REQUIRE(res->is<Map>());
        auto map = res->get<Map>().value();
        CHECK(map.size() == 2);
        REQUIRE(map.contains(key_ok));
        REQUIRE(map.contains(key_value));
        REQUIRE(callable->calls.size() == 1);
        REQUIRE(callable->calls.at(0).size() == 2);
        CHECK(callable->calls.at(0).at(0) == a);
        CHECK(callable->calls.at(0).at(1) == b);
    }

    SECTION("User error returns ok=false and error message")
    {
        auto callable = std::make_shared<Throwing_User_Callable>("boom"s);
        auto fn = Value::create(Function{callable});
        auto args = Value::create(Array{});

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
        auto callable = std::make_shared<Throwing_Internal_Callable>();
        auto fn = Value::create(Function{callable});
        auto args = Value::create(Array{});

        CHECK_THROWS_WITH(try_call->call({fn, args}),
                          ContainsSubstring("internal boom"));
    }
}
