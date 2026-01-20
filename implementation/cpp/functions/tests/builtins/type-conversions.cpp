#include "catch2/catch_test_macros.hpp"
#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/testing/dummy-callable.hpp>
#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <frost/builtin.hpp>

#include <memory>

using namespace frst;

using namespace std::literals;
using Catch::Matchers::ContainsSubstring;

TEST_CASE("Builtin to_string")
{
    Symbol_Table table;
    inject_builtins(table);
    auto to_string_val = table.lookup("to_string");
    REQUIRE(to_string_val->is<Function>());
    auto to_string = to_string_val->get<Function>().value();

    auto Null_ = Value::null();
    auto Int_ = Value::create(42_f);
    auto Float_ = Value::create(3.14);
    auto Bool_ = Value::create(true);
    auto String_ = Value::create("Hello!"s);
    auto Array_ = Value::create(frst::Array{
        Value::create(42_f),
        Value::create("hello"s),
    });
    auto Map_ = Value::create(frst::Map{
        {Value::create("key1"s), Value::create(42_f)},
        {Value::create(true), Value::create("value2"s)},
    });
    // AI-generated test additions by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto Function_ = Value::create(
        frst::Function{std::make_shared<frst::testing::Dummy_Callable>()});

    SECTION("Success")
    {
        CHECK(to_string->call({Null_})->get<String>().value() == "null");
        CHECK(to_string->call({Int_})->get<String>().value() == "42");
        CHECK(to_string->call({Float_})->get<String>().value().starts_with(
            "3.14"));
        CHECK(to_string->call({Bool_})->get<String>().value() == "true");
        CHECK(to_string->call({String_})->get<String>().value() == "Hello!");
        CHECK(to_string->call({Array_})->get<String>().value()
              == "[ 42, \"hello\" ]");
        CHECK(to_string->call({Map_})->get<String>().value()
              == R"({ [true]: "value2", ["key1"]: 42 })");
        CHECK(to_string->call({Function_})->get<String>().value()
              == "<Function>");
    }
}

// AI-generated test additions by Codex (GPT-5).
// Signed: Codex (GPT-5).

TEST_CASE("Builtin to_int")
{
    Symbol_Table table;
    inject_builtins(table);
    auto to_int_val = table.lookup("to_int");
    REQUIRE(to_int_val->is<Function>());
    auto to_int = to_int_val->get<Function>().value();

    SECTION("Injected")
    {
        CHECK(to_int_val->is<Function>());
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(to_int->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(to_int->call({Value::null(), Value::null()}),
                          ContainsSubstring("too many arguments"));
    }

    SECTION("Success")
    {
        auto res_int = to_int->call({Value::create(42_f)});
        REQUIRE(res_int->is<frst::Int>());
        CHECK(res_int->get<frst::Int>().value() == 42);

        auto res_float = to_int->call({Value::create(3.9)});
        REQUIRE(res_float->is<frst::Int>());
        CHECK(res_float->get<frst::Int>().value() == 3);

        auto res_string = to_int->call({Value::create("123"s)});
        REQUIRE(res_string->is<frst::Int>());
        CHECK(res_string->get<frst::Int>().value() == 123);
    }

    SECTION("Errors return Null")
    {
        CHECK(to_int->call({Value::create(true)})->is<frst::Null>());
        CHECK(to_int->call({Value::null()})->is<frst::Null>());
        CHECK(to_int->call({Value::create("3.14"s)})->is<frst::Null>());
    }
}

TEST_CASE("Builtin to_float")
{
    Symbol_Table table;
    inject_builtins(table);
    auto to_float_val = table.lookup("to_float");
    REQUIRE(to_float_val->is<Function>());
    auto to_float = to_float_val->get<Function>().value();

    SECTION("Injected")
    {
        CHECK(to_float_val->is<Function>());
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(to_float->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(to_float->call({Value::null(), Value::null()}),
                          ContainsSubstring("too many arguments"));
    }

    SECTION("Success")
    {
        auto res_int = to_float->call({Value::create(42_f)});
        REQUIRE(res_int->is<frst::Float>());
        CHECK(res_int->get<frst::Float>().value() == 42.0);

        auto res_float = to_float->call({Value::create(3.14)});
        REQUIRE(res_float->is<frst::Float>());
        CHECK(res_float->get<frst::Float>().value() == 3.14);

        auto res_string = to_float->call({Value::create("3.14"s)});
        REQUIRE(res_string->is<frst::Float>());
        CHECK(res_string->get<frst::Float>().value() == 3.14);
    }

    SECTION("Errors return Null")
    {
        CHECK(to_float->call({Value::create(true)})->is<frst::Null>());
        CHECK(to_float->call({Value::null()})->is<frst::Null>());
        CHECK(to_float->call({Value::create("0x10"s)})->is<frst::Null>());
    }
}
