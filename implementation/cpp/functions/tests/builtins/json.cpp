// AI-generated test by Codex (GPT-5).
// Signed: Codex (GPT-5).
#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_exception.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

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
} // namespace

TEST_CASE("Builtin parse_json")
{
    Symbol_Table table;
    inject_builtins(table);

    auto parse_json = lookup(table, "parse_json");

    SECTION("Injected")
    {
        CHECK(parse_json);
    }

    SECTION("Arity and type errors")
    {
        CHECK_THROWS_MATCHES(
            parse_json->call({}), Frost_User_Error,
            MessageMatches(ContainsSubstring("insufficient arguments")
                           && ContainsSubstring("requires at least 1")));
        CHECK_THROWS_MATCHES(parse_json->call({Value::create(1_f)}),
                             Frost_User_Error,
                             MessageMatches(ContainsSubstring("parse_json")
                                            && ContainsSubstring("String")
                                            && ContainsSubstring("Int")));
    }

    SECTION("Parses top-level values")
    {
        auto i = parse_json->call({Value::create("42"s)});
        REQUIRE(i->is<Int>());
        CHECK(i->get<Int>().value() == 42_f);

        auto f = parse_json->call({Value::create("1e3"s)});
        REQUIRE(f->is<Float>());
        CHECK(f->get<Float>().value() == Catch::Approx(1000.0));

        auto s = parse_json->call({Value::create("\"hi\""s)});
        REQUIRE(s->is<String>());
        CHECK(s->get<String>() == "hi");

        auto b = parse_json->call({Value::create("true"s)});
        REQUIRE(b->is<Bool>());
        CHECK(b->get<Bool>().value() == true);

        auto n = parse_json->call({Value::create("null"s)});
        REQUIRE(n->is<Null>());
    }

    SECTION("Allows comments and trailing commas")
    {
        auto arr = parse_json->call({Value::create(R"([1, 2,])"s)});
        REQUIRE(arr->is<Array>());
        CHECK(arr->raw_get<Array>().size() == 2);

        auto obj = parse_json->call({Value::create(
            R"({ "a": 1, // comment
                "b": 2, })"s)});
        REQUIRE(obj->is<Map>());
        CHECK(obj->raw_get<Map>().size() == 2);
    }

    SECTION("Parse errors are recoverable")
    {
        CHECK_THROWS_AS(parse_json->call({Value::create("{]"s)}),
                        Frost_Recoverable_Error);
    }

    SECTION("Out-of-range unsigned integers are recoverable errors")
    {
        CHECK_THROWS_AS(
            parse_json->call({Value::create("18446744073709551615"s)}),
            Frost_Recoverable_Error);
    }
}

TEST_CASE("Builtin to_json")
{
    Symbol_Table table;
    inject_builtins(table);

    auto to_json = lookup(table, "to_json");

    SECTION("Injected")
    {
        CHECK(to_json);
    }

    SECTION("Arity error")
    {
        CHECK_THROWS_MATCHES(
            to_json->call({}), Frost_User_Error,
            MessageMatches(ContainsSubstring("insufficient arguments")
                           && ContainsSubstring("requires at least 1")));
    }

    SECTION("Serializes primitive values compactly")
    {
        auto i = to_json->call({Value::create(42_f)});
        REQUIRE(i->is<String>());
        CHECK(i->get<String>() == "42");

        auto f = to_json->call({Value::create(1.5)});
        REQUIRE(f->is<String>());
        auto parse_json = lookup(table, "parse_json");
        auto parsed_f = parse_json->call({f});
        REQUIRE(parsed_f->is<Float>());
        CHECK(parsed_f->get<Float>().value() == Catch::Approx(1.5));

        auto b = to_json->call({Value::create(true)});
        REQUIRE(b->is<String>());
        CHECK(b->get<String>() == "true");

        auto n = to_json->call({Value::null()});
        REQUIRE(n->is<String>());
        CHECK(n->get<String>() == "null");

        auto s = to_json->call({Value::create("hi"s)});
        REQUIRE(s->is<String>());
        CHECK(s->get<String>() == "\"hi\"");
    }

    SECTION("Serializes arrays compactly")
    {
        auto arr = Value::create(Array{Value::create(1_f), Value::create(2_f)});
        auto json = to_json->call({arr});
        REQUIRE(json->is<String>());
        CHECK(json->get<String>() == "[1,2]");
    }

    SECTION("Non-string map keys are rejected")
    {
        auto map = Value::create(Map{{Value::create(1_f), Value::create(2_f)}});
        CHECK_THROWS_MATCHES(
            to_json->call({map}), Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("Map with non-string key")
                           && ContainsSubstring("1")));
    }

    SECTION("Functions are rejected")
    {
        auto fn = table.lookup("len");
        CHECK_THROWS_MATCHES(to_json->call({fn}), Frost_Recoverable_Error,
                             MessageMatches(ContainsSubstring(
                                 "Cannot serialize function to JSON")));
    }

    SECTION("Structures containing functions are rejected")
    {
        auto fn = table.lookup("len");
        auto arr = Value::create(Array{fn});
        CHECK_THROWS_AS(to_json->call({arr}), Frost_Recoverable_Error);
    }

    SECTION("Round-trip parse_json(to_json(value)) is deep-equal")
    {
        auto map = Value::create(Map{
            {Value::create("a"s),
             Value::create(Array{Value::create(1_f), Value::create(2_f)})},
            {Value::create("b"s), Value::create(true)},
        });

        auto json = to_json->call({map});
        REQUIRE(json->is<String>());

        auto parse_json = lookup(table, "parse_json");
        auto round = parse_json->call({json});
        auto eq = Value::deep_equal(map, round)->get<Bool>().value();
        CHECK(eq);
    }
}
