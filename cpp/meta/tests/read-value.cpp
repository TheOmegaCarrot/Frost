#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/meta.hpp>
#include <frost/symbol-table.hpp>
#include <frost/testing/stringmaker-specializations.hpp>
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

TEST_CASE("Builtin read_value")
{
    Symbol_Table table;
    inject_meta(table);

    auto read_value = lookup(table, "read_value");

    SECTION("Injected")
    {
        CHECK(read_value);
    }

    SECTION("Arity and type errors")
    {
        CHECK_THROWS_MATCHES(
            read_value->call({}), Frost_User_Error,
            MessageMatches(ContainsSubstring("insufficient arguments")
                           && ContainsSubstring("requires at least 1")));

        CHECK_THROWS_MATCHES(read_value->call({Value::create(42_f)}),
                             Frost_User_Error,
                             MessageMatches(ContainsSubstring("read_value")
                                            && ContainsSubstring("String")
                                            && ContainsSubstring("Int")));

        CHECK_THROWS_MATCHES(
            read_value->call({Value::create("42"s), Value::create("43"s)}),
            Frost_User_Error,
            MessageMatches(ContainsSubstring("too many arguments")
                           && ContainsSubstring("no more than 1")));
    }

    SECTION("Parses primitive values")
    {
        auto i = read_value->call({Value::create("42"s)});
        REQUIRE(i->is<Int>());
        CHECK(i->get<Int>().value() == 42_f);

        auto f = read_value->call({Value::create("3.14"s)});
        REQUIRE(f->is<Float>());
        CHECK(f->get<Float>().value() == Catch::Approx(3.14));

        auto s = read_value->call({Value::create(R"("hello")"s)});
        REQUIRE(s->is<String>());
        CHECK(s->get<String>() == "hello");

        auto b_true = read_value->call({Value::create("true"s)});
        REQUIRE(b_true->is<Bool>());
        CHECK(b_true->get<Bool>().value() == true);

        auto b_false = read_value->call({Value::create("false"s)});
        REQUIRE(b_false->is<Bool>());
        CHECK(b_false->get<Bool>().value() == false);

        auto n = read_value->call({Value::create("null"s)});
        CHECK(n->is<Null>());
    }

    SECTION("Parses negative numbers")
    {
        auto neg_int = read_value->call({Value::create("-7"s)});
        REQUIRE(neg_int->is<Int>());
        CHECK(neg_int->get<Int>().value() == -7_f);

        auto neg_float = read_value->call({Value::create("-1.5"s)});
        REQUIRE(neg_float->is<Float>());
        CHECK(neg_float->get<Float>().value() == Catch::Approx(-1.5));
    }

    SECTION("Parses arithmetic expressions")
    {
        auto sum = read_value->call({Value::create("1 + 2"s)});
        REQUIRE(sum->is<Int>());
        CHECK(sum->get<Int>().value() == 3_f);

        auto product = read_value->call({Value::create("3 * 4"s)});
        REQUIRE(product->is<Int>());
        CHECK(product->get<Int>().value() == 12_f);
    }

    SECTION("Parses arrays")
    {
        auto arr = read_value->call({Value::create("[1, 2, 3]"s)});
        REQUIRE(arr->is<Array>());
        CHECK(arr->raw_get<Array>().size() == 3);
    }

    SECTION("Parses maps")
    {
        auto map = read_value->call({Value::create("{foo: 1, bar: 2}"s)});
        REQUIRE(map->is<Map>());
        CHECK(map->raw_get<Map>().size() == 2);
    }

    SECTION("Parses nested structures")
    {
        auto nested =
            read_value->call({Value::create(R"({x: [1, 2], y: "hi"})"s)});
        REQUIRE(nested->is<Map>());
    }

    SECTION("Rejects name lookup")
    {
        CHECK_THROWS_MATCHES(
            read_value->call({Value::create("foo"s)}), Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("Invalid node in Frost Data")));
    }

    SECTION("Rejects function call")
    {
        CHECK_THROWS_MATCHES(
            read_value->call({Value::create("len([])"s)}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("Invalid node in Frost Data")));
    }

    SECTION("Rejects lambda")
    {
        CHECK_THROWS_MATCHES(
            read_value->call({Value::create("fn x -> x"s)}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("Invalid node in Frost Data")));
    }

    SECTION("Unsafe children of safe nodes are still rejected")
    {
        // Binop is data_safe, but its children are still walked
        CHECK_THROWS_MATCHES(
            read_value->call({Value::create("1 + foo"s)}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("Invalid node in Frost Data")));

        // Unop is data_safe, but its child is still walked
        CHECK_THROWS_MATCHES(
            read_value->call({Value::create("-foo"s)}), Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("Invalid node in Frost Data")));

        // Unsafe node nested inside an array
        CHECK_THROWS_MATCHES(
            read_value->call({Value::create("[1, foo, 3]"s)}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("Invalid node in Frost Data")));
    }

    SECTION("Invalid syntax is a recoverable error")
    {
        CHECK_THROWS_MATCHES(
            read_value->call({Value::create("[1, 2"s)}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("Invalid Frost Data")));
    }
}
