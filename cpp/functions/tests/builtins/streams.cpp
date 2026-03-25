#include <catch2/catch_all.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

using namespace frst;
using namespace std::literals;

namespace
{

Function get_map_fn(const Value_Ptr& map_val, std::string_view name)
{
    REQUIRE(map_val->is<Map>());
    const auto& map = map_val->raw_get<Map>();
    auto key = Value::create(std::string{name});
    for (const auto& [k, v] : map)
    {
        if (Value::equal(k, key)->get<Bool>().value())
        {
            REQUIRE(v);
            REQUIRE(v->is<Function>());
            return v->get<Function>().value();
        }
    }
    FAIL("Missing map entry");
    return Function{};
}

} // namespace

TEST_CASE("Builtin stdin")
{
    Symbol_Table table;
    inject_builtins(table);

    auto stdin_val = table.lookup("stdin");
    REQUIRE(stdin_val->is<Map>());

    SECTION("Injected")
    {
        CHECK(stdin_val->is<Map>());
    }

    SECTION("stdin functions are present")
    {
        auto read_line = get_map_fn(stdin_val, "read_line");
        auto read_one = get_map_fn(stdin_val, "read_one");
        auto read = get_map_fn(stdin_val, "read");

        CHECK(read_line);
        CHECK(read_one);
        CHECK(read);
    }
}

TEST_CASE("Builtin stderr")
{
    Symbol_Table table;
    inject_builtins(table);

    auto stderr_val = table.lookup("stderr");
    REQUIRE(stderr_val->is<Map>());

    SECTION("stderr functions are present")
    {
        auto write = get_map_fn(stderr_val, "write");
        auto writeln = get_map_fn(stderr_val, "writeln");

        CHECK(write);
        CHECK(writeln);
    }
}
