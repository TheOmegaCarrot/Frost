#include <catch2/catch_test_macros.hpp>

#include <frost/symbol-table.hpp>

using frst::Symbol_Table;
using frst::Value;

using namespace frst::literals;
using namespace std::literals;

TEST_CASE("Define and Lookup")
{
    Symbol_Table table1;
    Symbol_Table table2(&table1);

    REQUIRE(table1.define("foo", Value::create("foo1"s)).has_value());
    REQUIRE(table1.define("bar", Value::create("bar1"s)).has_value());

    REQUIRE(table2.define("bar", Value::create("bar2"s)).has_value());
    REQUIRE(table2.define("beep", Value::create("beep2"s)).has_value());

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
        auto res = table2.define("beep", Value::create("uh oh"s));
        REQUIRE_FALSE(res.has_value());
        CHECK(res.error() == "Cannot define beep as it is already defined");
        CHECK(table2.lookup("beep")->get<frst::String>() == "beep2");
    }
}
