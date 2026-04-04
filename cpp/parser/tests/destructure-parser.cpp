#include <catch2/catch_test_macros.hpp>

#include <frost/ast.hpp>
#include <frost/symbol-table.hpp>
#include <frost/testing/stringmaker-specializations.hpp>
#include <frost/value.hpp>

#include <lexy/action/parse.hpp>
#include <lexy/callback.hpp>
#include <lexy/input/string_input.hpp>

#include "../grammar.hpp"

using namespace frst::literals;
using namespace std::literals;

namespace
{
struct Program_Root
{
    static constexpr auto rule = lexy::dsl::p<frst::grammar::program>;
    static constexpr auto value =
        lexy::forward<std::vector<frst::ast::Statement::Ptr>>;
};

auto parse(std::string_view input)
{
    auto src = lexy::string_input<lexy::utf8_encoding>(input);
    frst::grammar::reset_parse_state(src);
    return lexy::parse<Program_Root>(src, lexy::noop);
}

void run(std::string_view code, frst::Symbol_Table& table)
{
    auto result = parse(code);
    REQUIRE(result);
    auto program = std::move(result).value();
    frst::Execution_Context ctx{.symbols = table};
    for (const auto& stmt : program)
        stmt->execute(ctx);
}

frst::Value_Ptr lookup(frst::Symbol_Table& table, const std::string& name)
{
    return table.lookup(name);
}
} // namespace

TEST_CASE("Parser Recursive Destructuring")
{
    SECTION("Nested array destructure")
    {
        frst::Symbol_Table table;
        run("def [a, [b, c]] = [1, [2, 3]]", table);

        CHECK(lookup(table, "a")->get<frst::Int>().value() == 1_f);
        CHECK(lookup(table, "b")->get<frst::Int>().value() == 2_f);
        CHECK(lookup(table, "c")->get<frst::Int>().value() == 3_f);
    }

    SECTION("Map with array value destructure")
    {
        frst::Symbol_Table table;
        run("def {foo: [a, b]} = {foo: [1, 2]}", table);

        CHECK(lookup(table, "a")->get<frst::Int>().value() == 1_f);
        CHECK(lookup(table, "b")->get<frst::Int>().value() == 2_f);
    }

    SECTION("Array with map element destructure")
    {
        frst::Symbol_Table table;
        run("def [a, {foo}] = [1, {foo: 2}]", table);

        CHECK(lookup(table, "a")->get<frst::Int>().value() == 1_f);
        CHECK(lookup(table, "foo")->get<frst::Int>().value() == 2_f);
    }

    SECTION("Nested map destructure")
    {
        frst::Symbol_Table table;
        run("def {a: {b: c}} = {a: {b: 42}}", table);

        CHECK(lookup(table, "c")->get<frst::Int>().value() == 42_f);
    }

    SECTION("Discards in nested destructure")
    {
        frst::Symbol_Table table;
        run("def [_, [_, x]] = [1, [2, 3]]", table);

        CHECK(lookup(table, "x")->get<frst::Int>().value() == 3_f);
        CHECK_FALSE(table.has("_"));
    }

    SECTION("Rest in nested array destructure")
    {
        frst::Symbol_Table table;
        run("def [a, [b, ...rest]] = [1, [2, 3, 4]]", table);

        CHECK(lookup(table, "a")->get<frst::Int>().value() == 1_f);
        CHECK(lookup(table, "b")->get<frst::Int>().value() == 2_f);

        auto rest = lookup(table, "rest");
        REQUIRE(rest->is<frst::Array>());
        auto rest_arr = rest->get<frst::Array>().value();
        REQUIRE(rest_arr.size() == 2);
        CHECK(rest_arr[0]->get<frst::Int>().value() == 3_f);
        CHECK(rest_arr[1]->get<frst::Int>().value() == 4_f);
    }

    SECTION("Rest in map value destructure")
    {
        frst::Symbol_Table table;
        run("def {foo: [_, ...rest]} = {foo: [1, 2, 3]}", table);

        auto rest = lookup(table, "rest");
        REQUIRE(rest->is<frst::Array>());
        auto rest_arr = rest->get<frst::Array>().value();
        REQUIRE(rest_arr.size() == 2);
        CHECK(rest_arr[0]->get<frst::Int>().value() == 2_f);
        CHECK(rest_arr[1]->get<frst::Int>().value() == 3_f);
    }

    SECTION("Export with nested destructure")
    {
        frst::Symbol_Table table;
        run("export def [a, [b, c]] = [1, [2, 3]]", table);

        CHECK(lookup(table, "a")->get<frst::Int>().value() == 1_f);
        CHECK(lookup(table, "b")->get<frst::Int>().value() == 2_f);
        CHECK(lookup(table, "c")->get<frst::Int>().value() == 3_f);
    }

    SECTION("Empty nested array destructure")
    {
        frst::Symbol_Table table;
        run("def [a, []] = [1, []]", table);

        CHECK(lookup(table, "a")->get<frst::Int>().value() == 1_f);
    }

    SECTION("Deeply nested destructure")
    {
        frst::Symbol_Table table;
        run("def [[[x]]] = [[[42]]]", table);

        CHECK(lookup(table, "x")->get<frst::Int>().value() == 42_f);
    }

    SECTION("Map shorthand inside array destructure")
    {
        frst::Symbol_Table table;
        run("def [{foo, bar}] = [{foo: 1, bar: 2}]", table);

        CHECK(lookup(table, "foo")->get<frst::Int>().value() == 1_f);
        CHECK(lookup(table, "bar")->get<frst::Int>().value() == 2_f);
    }

    SECTION("Computed key with nested pattern")
    {
        frst::Symbol_Table table;
        run("def {[1+1]: [a, b]} = {[2]: [3, 4]}", table);

        CHECK(lookup(table, "a")->get<frst::Int>().value() == 3_f);
        CHECK(lookup(table, "b")->get<frst::Int>().value() == 4_f);
    }

    SECTION("Missing map key in nested throws on type mismatch")
    {
        frst::Symbol_Table table;
        // foo key is missing, so the array destructure receives null,
        // which should throw because null is not an Array
        CHECK_THROWS(run("def {foo: [a, b]} = {bar: [1, 2]}", table));
    }

    SECTION("Invalid: rest not at end in nested")
    {
        CHECK_FALSE(parse("def [a, [...rest, b]] = x"));
    }

    SECTION("Invalid: pattern where rest name should be")
    {
        CHECK_FALSE(parse("def [a, ...[b, c]] = x"));
    }

    SECTION("Backwards compatible: flat array destructure still works")
    {
        frst::Symbol_Table table;
        run("def [a, b, c] = [1, 2, 3]", table);

        CHECK(lookup(table, "a")->get<frst::Int>().value() == 1_f);
        CHECK(lookup(table, "b")->get<frst::Int>().value() == 2_f);
        CHECK(lookup(table, "c")->get<frst::Int>().value() == 3_f);
    }

    SECTION("Backwards compatible: flat map destructure still works")
    {
        frst::Symbol_Table table;
        run("def {foo, bar: baz} = {foo: 1, bar: 2}", table);

        CHECK(lookup(table, "foo")->get<frst::Int>().value() == 1_f);
        CHECK(lookup(table, "baz")->get<frst::Int>().value() == 2_f);
    }

    SECTION("Backwards compatible: simple def still works")
    {
        frst::Symbol_Table table;
        run("def x = 42", table);

        CHECK(lookup(table, "x")->get<frst::Int>().value() == 42_f);
    }
}
