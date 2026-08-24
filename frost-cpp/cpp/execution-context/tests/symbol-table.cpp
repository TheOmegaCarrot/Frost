#include <catch2/catch_test_macros.hpp>

#include <frost/symbol-table.hpp>

using frst::Symbol_Table;
using frst::Value;

using namespace frst::literals;
using namespace std::literals;

TEST_CASE("Symbol Table")
{
    Symbol_Table table1;
    Symbol_Table table2(&table1);

    REQUIRE_NOTHROW(table1.define("foo", Value::create("foo1"s)));
    REQUIRE_NOTHROW(table1.define("bar", Value::create("bar1"s)));

    REQUIRE_NOTHROW(table2.define("bar", Value::create("bar2"s)));
    REQUIRE_NOTHROW(table2.define("beep", Value::create("beep2"s)));

    SECTION("Direct Fail")
    {
        CHECK_THROWS(table1.lookup("nope"));
    }

    SECTION("Failover Fail")
    {
        CHECK_THROWS(table2.lookup("nope"));
    }

    SECTION("Direct Lookup")
    {
        CHECK(table1.lookup("foo")->get<frst::String>() == "foo1");
    }

    SECTION("Failover Lookup")
    {
        CHECK(table2.lookup("beep")->get<frst::String>() == "beep2");
    }

    SECTION("Shadowed Lookup")
    {
        CHECK(table2.lookup("bar")->get<frst::String>() == "bar2");
    }

    SECTION("Redefine")
    {
        CHECK_THROWS(table2.define("beep", Value::create("uh oh"s)));
        CHECK(table2.lookup("beep")->get<frst::String>() == "beep2");
    }

    SECTION("Has")
    {
        CHECK(table1.has("foo"));
        CHECK(table1.has("bar"));

        CHECK(table2.has("foo"));
        CHECK(table2.has("bar"));
        CHECK(table2.has("beep"));

        CHECK_FALSE(table1.has("beep"));
    }
}
