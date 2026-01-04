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

    table1.define("foo", Value::create("foo1"s));
    table1.define("bar", Value::create("bar1"s));

    table2.define("bar", Value::create("bar2"s));
    table2.define("beep", Value::create("beep2"s));

    SECTION("Direct Fail")
    {
        CHECK_FALSE(table1.lookup("nope").has_value());
    }

    SECTION("Failover Fail")
    {
        CHECK_FALSE(table2.lookup("nope").has_value());
    }

    SECTION("Direct Lookup")
    {
        CHECK(table1.lookup("foo").value()->get<frst::String>() == "foo1");
    }

    SECTION("Failover Lookup")
    {
        CHECK(table2.lookup("beep").value()->get<frst::String>() == "beep2");
    }

    SECTION("Shadowed Lookup")
    {
        CHECK(table2.lookup("bar").value()->get<frst::String>() == "bar2");
    }
}
