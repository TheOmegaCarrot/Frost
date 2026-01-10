#include "catch2/catch_test_macros.hpp"
#include <catch2/catch_all.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <frost/builtin.hpp>

using namespace frst;

using namespace std::literals;

TEST_CASE("Builtin to_string")
{
    Symbol_Table table;
    inject_builtins(table);
    auto to_string_val = table.lookup("to_string");
    REQUIRE(to_string_val->is<Function>());
    auto to_string = to_string_val->get<Function>().value();

    auto Null_ = Value::create();
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

    SECTION("Success")
    {
        CHECK(to_string->call({Null_})->get<String>().value() == "null");
        CHECK(to_string->call({Int_})->get<String>().value() == "42");
        CHECK(to_string->call({Float_})->get<String>().value().starts_with(
            "3.14"));
        CHECK(to_string->call({Bool_})->get<String>().value() == "true");
        CHECK(to_string->call({String_})->get<String>().value() == "Hello!");
        CHECK(to_string->call({Array_})->get<String>().value() ==
              "[ 42, \"hello\" ]");
        CHECK(to_string->call({Map_})->get<String>().value() ==
              R"({ [true]: "value2", ["key1"]: 42 })");
    }
}
