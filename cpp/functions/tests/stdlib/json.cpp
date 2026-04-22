#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_exception.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/builtin.hpp>
#include <frost/stdlib.hpp>
#include <frost/value.hpp>

using namespace frst;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;
using Catch::Matchers::MessageMatches;

namespace
{

Map json_module()
{
    Stdlib_Registry_Builder builder;
    register_module_json(builder);
    auto registry = std::move(builder).build();
    auto module = registry.lookup_module("std.json");
    REQUIRE(module.has_value());
    REQUIRE(module.value()->is<Map>());
    return module.value()->raw_get<Map>();
}

Function lookup(const Map& mod, const std::string& name)
{
    auto key = Value::create(String{name});
    auto it = mod.find(key);
    REQUIRE(it != mod.end());
    REQUIRE(it->second->is<Function>());
    return it->second->raw_get<Function>();
}

} // namespace

TEST_CASE("std.json decode")
{
    auto mod = json_module();
    auto decode = lookup(mod, "decode");

    SECTION("Registered in module")
    {
        CHECK(decode);
    }

    SECTION("Arity and type errors")
    {
        CHECK_THROWS_MATCHES(
            decode->call({}), Frost_User_Error,
            MessageMatches(ContainsSubstring("insufficient arguments")
                           && ContainsSubstring("requires at least 1")));
        CHECK_THROWS_MATCHES(decode->call({Value::create(1_f)}),
                             Frost_User_Error,
                             MessageMatches(ContainsSubstring("json.decode")
                                            && ContainsSubstring("String")
                                            && ContainsSubstring("Int")));
    }

    SECTION("Parses top-level values")
    {
        auto i = decode->call({Value::create("42"s)});
        REQUIRE(i->is<Int>());
        CHECK(i->get<Int>().value() == 42_f);

        auto f = decode->call({Value::create("1e3"s)});
        REQUIRE(f->is<Float>());
        CHECK(f->get<Float>().value() == Catch::Approx(1000.0));

        auto s = decode->call({Value::create("\"hi\""s)});
        REQUIRE(s->is<String>());
        CHECK(s->get<String>() == "hi");

        auto b = decode->call({Value::create("true"s)});
        REQUIRE(b->is<Bool>());
        CHECK(b->get<Bool>().value() == true);

        auto n = decode->call({Value::create("null"s)});
        REQUIRE(n->is<Null>());
    }

    SECTION("Allows comments and trailing commas")
    {
        auto arr = decode->call({Value::create(R"([1, 2,])"s)});
        REQUIRE(arr->is<Array>());
        CHECK(arr->raw_get<Array>().size() == 2);

        auto obj = decode->call({Value::create(
            R"({ "a": 1, // comment
                "b": 2, })"s)});
        REQUIRE(obj->is<Map>());
        CHECK(obj->raw_get<Map>().size() == 2);
    }

    SECTION("Parse errors are recoverable")
    {
        CHECK_THROWS_AS(decode->call({Value::create("{]"s)}),
                        Frost_Recoverable_Error);
    }

    SECTION("Out-of-range unsigned integers are recoverable errors")
    {
        CHECK_THROWS_AS(decode->call({Value::create("18446744073709551615"s)}),
                        Frost_Recoverable_Error);
    }
}

TEST_CASE("std.json encode")
{
    auto mod = json_module();
    auto encode = lookup(mod, "encode");

    SECTION("Registered in module")
    {
        CHECK(encode);
    }

    SECTION("Arity error")
    {
        CHECK_THROWS_MATCHES(
            encode->call({}), Frost_User_Error,
            MessageMatches(ContainsSubstring("insufficient arguments")
                           && ContainsSubstring("requires at least 1")));
    }

    SECTION("Serializes primitive values compactly")
    {
        auto i = encode->call({Value::create(42_f)});
        REQUIRE(i->is<String>());
        CHECK(i->get<String>() == "42");

        auto f = encode->call({Value::create(1.5)});
        REQUIRE(f->is<String>());
        auto decode = lookup(mod, "decode");
        auto parsed_f = decode->call({f});
        REQUIRE(parsed_f->is<Float>());
        CHECK(parsed_f->get<Float>().value() == Catch::Approx(1.5));

        auto b = encode->call({Value::create(true)});
        REQUIRE(b->is<String>());
        CHECK(b->get<String>() == "true");

        auto n = encode->call({Value::null()});
        REQUIRE(n->is<String>());
        CHECK(n->get<String>() == "null");

        auto s = encode->call({Value::create("hi"s)});
        REQUIRE(s->is<String>());
        CHECK(s->get<String>() == "\"hi\"");
    }

    SECTION("Serializes arrays compactly")
    {
        auto arr = Value::create(Array{Value::create(1_f), Value::create(2_f)});
        auto json = encode->call({arr});
        REQUIRE(json->is<String>());
        CHECK(json->get<String>() == "[1,2]");
    }

    SECTION("Non-String Map keys are rejected")
    {
        auto map = Value::create(Map{{Value::create(1_f), Value::create(2_f)}});
        CHECK_THROWS_MATCHES(
            encode->call({map}), Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("Map with non-String key")
                           && ContainsSubstring("1")));
    }

    SECTION("Functions are rejected")
    {
        Symbol_Table table;
        inject_builtins(table);
        auto fn = table.lookup("len");
        CHECK_THROWS_MATCHES(encode->call({fn}), Frost_Recoverable_Error,
                             MessageMatches(ContainsSubstring(
                                 "Cannot serialize Function to JSON")));
    }

    SECTION("Structures containing functions are rejected")
    {
        Symbol_Table table;
        inject_builtins(table);
        auto fn = table.lookup("len");
        auto arr = Value::create(Array{fn});
        CHECK_THROWS_AS(encode->call({arr}), Frost_Recoverable_Error);
    }

    SECTION("Round-trip decode(encode(value)) is deep-equal")
    {
        auto map = Value::create(Map{
            {Value::create("a"s),
             Value::create(Array{Value::create(1_f), Value::create(2_f)})},
            {Value::create("b"s), Value::create(true)},
        });

        auto json = encode->call({map});
        REQUIRE(json->is<String>());

        auto decode = lookup(mod, "decode");
        auto round = decode->call({json});
        auto eq = Value::equal(map, round)->get<Bool>().value();
        CHECK(eq);
    }
}

TEST_CASE("std.json encode_pretty")
{
    auto mod = json_module();
    auto encode_pretty = lookup(mod, "encode_pretty");

    SECTION("Arity and type errors")
    {
        CHECK_THROWS_MATCHES(
            encode_pretty->call({}), Frost_User_Error,
            MessageMatches(ContainsSubstring("insufficient arguments")
                           && ContainsSubstring("requires at least 2")));
        CHECK_THROWS_MATCHES(
            encode_pretty->call({Value::create(1_f)}), Frost_User_Error,
            MessageMatches(ContainsSubstring("insufficient arguments")));
        CHECK_THROWS_MATCHES(
            encode_pretty->call({Value::create(1_f), Value::create("x"s)}),
            Frost_User_Error,
            MessageMatches(ContainsSubstring("indent")
                           && ContainsSubstring("got String")));
    }

    SECTION("Negative indent is rejected")
    {
        CHECK_THROWS_MATCHES(
            encode_pretty->call({Value::null(), Value::create(-1_f)}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("non-negative")));
    }

    SECTION("Primitive values are unaffected by indent")
    {
        auto two = Value::create(2_f);
        CHECK(encode_pretty->call({Value::null(), two})->raw_get<String>()
              == "null");
        CHECK(encode_pretty->call({Value::create(42_f), two})->raw_get<String>()
              == "42");
        CHECK(encode_pretty->call({Value::create(true), two})->raw_get<String>()
              == "true");
        CHECK(
            encode_pretty->call({Value::create(1.5), two})->raw_get<String>()
            == "1.5E0");
        CHECK(
            encode_pretty->call({Value::create("hi"s), two})->raw_get<String>()
            == "\"hi\"");
    }

    SECTION("Empty containers stay compact")
    {
        auto two = Value::create(2_f);
        CHECK(encode_pretty->call({Value::create(Array{}), two})
                  ->raw_get<String>()
              == "[]");
        CHECK(encode_pretty
                  ->call({Value::create(Value::trusted, Map{}), two})
                  ->raw_get<String>()
              == "{}");
    }

    SECTION("Array with indent 2")
    {
        auto arr = Value::create(
            Array{Value::create(1_f), Value::create(2_f), Value::create(3_f)});
        auto result =
            encode_pretty->call({arr, Value::create(2_f)})->raw_get<String>();
        CHECK(result == "[\n  1,\n  2,\n  3\n]");
    }

    SECTION("Object with indent 4")
    {
        auto map = Value::create(
            Map{{Value::create("a"s), Value::create(1_f)},
                {Value::create("b"s), Value::create(2_f)}});
        auto result =
            encode_pretty->call({map, Value::create(4_f)})->raw_get<String>();
        CHECK(result == "{\n    \"a\": 1,\n    \"b\": 2\n}");
    }

    SECTION("Nested structures")
    {
        auto inner = Value::create(
            Array{Value::create(1_f), Value::create(2_f)});
        auto map = Value::create(
            Map{{Value::create("x"s), inner}});
        auto result =
            encode_pretty->call({map, Value::create(2_f)})->raw_get<String>();
        CHECK(result == "{\n  \"x\": [\n    1,\n    2\n  ]\n}");
    }

    SECTION("Indent 0 uses newlines but no padding")
    {
        auto arr = Value::create(
            Array{Value::create(1_f), Value::create(2_f)});
        auto result =
            encode_pretty->call({arr, Value::create(0_f)})->raw_get<String>();
        CHECK(result == "[\n1,\n2\n]");
    }

    SECTION("Round-trip through decode preserves values")
    {
        auto decode = lookup(mod, "decode");
        auto map = Value::create(
            Map{{Value::create("a"s), Value::create(Array{
                                          Value::create(1_f),
                                          Value::create("two"s),
                                          Value::null(),
                                      })}});
        auto json = encode_pretty->call({map, Value::create(2_f)});
        auto round = decode->call({json});
        CHECK(Value::equal(map, round)->raw_get<Bool>());
    }

    SECTION("Non-String Map keys are rejected")
    {
        auto map = Value::create(Map{{Value::create(1_f), Value::create(2_f)}});
        CHECK_THROWS_AS(
            encode_pretty->call({map, Value::create(2_f)}),
            Frost_Recoverable_Error);
    }

    SECTION("Functions are rejected")
    {
        Symbol_Table table;
        inject_builtins(table);
        auto fn = table.lookup("len");
        CHECK_THROWS_AS(
            encode_pretty->call({fn, Value::create(2_f)}),
            Frost_Recoverable_Error);
    }
}
