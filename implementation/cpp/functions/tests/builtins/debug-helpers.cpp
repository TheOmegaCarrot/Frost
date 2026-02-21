#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/testing/dummy-callable.hpp>
#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <frost/builtin.hpp>

using namespace frst;

using namespace std::literals;
using Catch::Matchers::ContainsSubstring;
using Catch::Matchers::Equals;

TEST_CASE("Builtin debug_dump")
{
    // AI-generated test additions by Codex (GPT-5).
    // Signed: Codex (GPT-5).

    Symbol_Table table;
    inject_builtins(table);
    auto debug_val = table.lookup("debug_dump");
    REQUIRE(debug_val->is<Function>());
    auto debug_dump = debug_val->get<Function>().value();

    SECTION("Injected")
    {
        CHECK(debug_val->is<Function>());
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(debug_dump->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(debug_dump->call({Value::null(), Value::null()}),
                          ContainsSubstring("too many arguments"));
    }

    SECTION("Non-function values use to_string")
    {
        std::vector<Value_Ptr> values{
            Value::null(), // Null
            Value::create(42_f),
            Value::create(3.14),
            Value::create(true),
            Value::create(frst::Array{Value::create(1_f)}),
            Value::create(frst::Map{
                {Value::create("k"s), Value::create(1_f)},
            }),
        };

        for (const auto& val : values)
        {
            auto res = debug_dump->call({val});
            REQUIRE(res->is<frst::String>());
            CHECK(res->get<frst::String>().value()
                  == val->to_internal_string());
        }

        SECTION("But strings are debug-printed")
        {
            auto value = Value::create("hello\0there"s);
            auto res = debug_dump->call({value});
            REQUIRE(res->is<String>());
            CHECK(res->get<frst::String>().value() == "\"hello\\x00there\"");
        }
    }

    SECTION("Builtin function debug_dump")
    {
        auto func = Value::create(Function{std::make_shared<Builtin>(
            [](builtin_args_t) {
                return Value::null();
            },
            "dbg", Builtin::Arity{0, 0})});

        auto res = debug_dump->call({func});
        REQUIRE(res->is<frst::String>());
        CHECK(res->get<frst::String>().value() == "<builtin:dbg>");
    }

    SECTION("Custom callable debug_dump")
    {
        // AI-generated test additions by Codex (GPT-5).
        // Signed: Codex (GPT-5).
        auto func = Value::create(
            Function{std::make_shared<frst::testing::Dummy_Callable>()});

        auto res = debug_dump->call({func});
        REQUIRE(res->is<frst::String>());
        CHECK(res->get<frst::String>().value() == "<dummy>");
    }
}

TEST_CASE("Assert")
{
    // AI-generated test additions by Codex (GPT-5).
    // Signed: Codex (GPT-5).

    Symbol_Table table;
    inject_builtins(table);
    auto assert_val = table.lookup("assert");
    REQUIRE(assert_val->is<Function>());
    auto assert_fn = assert_val->get<Function>().value();

    SECTION("Injected")
    {
        CHECK(assert_val->is<Function>());
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(assert_fn->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(
            assert_fn->call({Value::null(), Value::null(), Value::null()}),
            ContainsSubstring("too many arguments"));
    }

    SECTION("Truthy values pass through (pointer-equal)")
    {
        auto v_int_zero = Value::create(0_f);
        auto v_empty_str = Value::create(""s);
        auto v_empty_arr = Value::create(frst::Array{});
        auto v_empty_map = Value::create(frst::Map{});
        auto v_true = Value::create(true);

        CHECK(assert_fn->call({v_int_zero}) == v_int_zero);
        CHECK(assert_fn->call({v_empty_str}) == v_empty_str);
        CHECK(assert_fn->call({v_empty_arr}) == v_empty_arr);
        CHECK(assert_fn->call({v_empty_map}) == v_empty_map);
        CHECK(assert_fn->call({v_true}) == v_true);
    }

    SECTION("Falsy values fail with exact default message")
    {
        auto v_null = Value::null();
        auto v_false = Value::create(false);

        CHECK_THROWS_WITH(assert_fn->call({v_null}),
                          Equals("Failed assertion"));
        CHECK_THROWS_WITH(assert_fn->call({v_false}),
                          Equals("Failed assertion"));
    }

    SECTION("Custom failure message is included")
    {
        auto v_false = Value::create(false);
        auto msg = Value::create("useful info"s);

        CHECK_THROWS_WITH(assert_fn->call({v_false, msg}),
                          ContainsSubstring("useful info"));
    }

    SECTION("Second argument must be string")
    {
        auto v_true = Value::create(true);
        auto bad_msg = Value::create(123_f);

        try
        {
            assert_fn->call({v_true, bad_msg});
            FAIL("Expected type error");
        }
        catch (const Frost_User_Error& err)
        {
            const std::string msg = err.what();
            CHECK_THAT(msg, ContainsSubstring("Function assert"));
            CHECK_THAT(msg, ContainsSubstring("String"));
            CHECK_THAT(msg, ContainsSubstring("argument 2"));
            CHECK_THAT(msg, ContainsSubstring("got Int"));
        }
    }

    SECTION("Two-argument form returns first arg by pointer")
    {
        auto v_val = Value::create(42_f);
        auto msg = Value::create("ignored"s);

        auto res = assert_fn->call({v_val, msg});
        CHECK(res == v_val);
    }
}
