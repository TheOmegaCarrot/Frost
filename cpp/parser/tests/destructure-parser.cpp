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

namespace
{
// Helper: find all nodes matching a label prefix via walk()
std::vector<const frst::ast::AST_Node*> find_nodes(
    const frst::ast::Statement::Ptr& stmt, std::string_view prefix)
{
    std::vector<const frst::ast::AST_Node*> result;
    for (auto* n : stmt->walk())
        if (n->node_label().starts_with(prefix))
            result.push_back(n);
    return result;
}
} // namespace

TEST_CASE("Parser Destructure Source Ranges")
{
    SECTION("Leaf identifier range")
    {
        // "def x = 1"
        //      ^
        //  col 5
        auto result = parse("def x = 1");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        auto leaves = find_nodes(program[0], "Destructure_Leaf");
        REQUIRE(leaves.size() == 1);
        auto range = leaves[0]->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 5);
        CHECK(range.end.line == 1);
        CHECK(range.end.column == 5);
    }

    SECTION("Array destructure range spans brackets")
    {
        // "def [a, b] = x"
        //      ^    ^
        //  col 5  col 10
        auto result = parse("def [a, b] = x");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        auto arrays = find_nodes(program[0], "Destructure_Array");
        REQUIRE(arrays.size() == 1);
        auto range = arrays[0]->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 5);
        CHECK(range.end.line == 1);
        CHECK(range.end.column == 10);
    }

    SECTION("Map destructure range spans braces")
    {
        // "def {foo} = x"
        //      ^   ^
        //  col 5  col 9
        auto result = parse("def {foo} = x");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        auto maps = find_nodes(program[0], "Destructure_Map");
        REQUIRE(maps.size() == 1);
        auto range = maps[0]->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 5);
        CHECK(range.end.line == 1);
        CHECK(range.end.column == 9);
    }

    SECTION("Nested array ranges are independent")
    {
        // "def [a, [b, c]] = x"
        //      ^         ^
        //  col 5       col 15
        //          ^      ^
        //      col 9  col 14
        auto result = parse("def [a, [b, c]] = x");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        auto arrays = find_nodes(program[0], "Destructure_Array");
        REQUIRE(arrays.size() == 2);

        auto outer = arrays[0]->source_range();
        CHECK(outer.begin.column == 5);
        CHECK(outer.end.column == 15);

        auto inner = arrays[1]->source_range();
        CHECK(inner.begin.column == 9);
        CHECK(inner.end.column == 14);
    }

    SECTION("Nested leaf ranges in array")
    {
        // "def [a, [b, c]] = x"
        //      ^   ^   ^
        //  col 6  10  13
        auto result = parse("def [a, [b, c]] = x");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        auto leaves = find_nodes(program[0], "Destructure_Leaf");
        REQUIRE(leaves.size() == 3);

        CHECK(leaves[0]->source_range().begin.column == 6);
        CHECK(leaves[1]->source_range().begin.column == 10);
        CHECK(leaves[2]->source_range().begin.column == 13);
    }

    SECTION("Map inside array range")
    {
        // "def [a, {foo: b}] = x"
        //      ^           ^
        //  col 5        col 17
        //          ^      ^
        //      col 9  col 16
        auto result = parse("def [a, {foo: b}] = x");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        auto arrays = find_nodes(program[0], "Destructure_Array");
        REQUIRE(arrays.size() == 1);
        CHECK(arrays[0]->source_range().begin.column == 5);
        CHECK(arrays[0]->source_range().end.column == 17);

        auto maps = find_nodes(program[0], "Destructure_Map");
        REQUIRE(maps.size() == 1);
        CHECK(maps[0]->source_range().begin.column == 9);
        CHECK(maps[0]->source_range().end.column == 16);
    }

    SECTION("Array inside map range")
    {
        // "def {foo: [a, b]} = x"
        //      ^           ^
        //  col 5         col 17
        //            ^      ^
        //        col 11  col 16
        auto result = parse("def {foo: [a, b]} = x");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        auto maps = find_nodes(program[0], "Destructure_Map");
        REQUIRE(maps.size() == 1);
        CHECK(maps[0]->source_range().begin.column == 5);
        CHECK(maps[0]->source_range().end.column == 17);

        auto arrays = find_nodes(program[0], "Destructure_Array");
        REQUIRE(arrays.size() == 1);
        CHECK(arrays[0]->source_range().begin.column == 11);
        CHECK(arrays[0]->source_range().end.column == 16);
    }

    SECTION("Map shorthand leaf gets identifier range")
    {
        // "def {foo} = x"
        //       ^
        //   col 6-8
        auto result = parse("def {foo} = x");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        auto leaves = find_nodes(program[0], "Destructure_Leaf");
        REQUIRE(leaves.size() == 1);
        auto range = leaves[0]->source_range();
        CHECK(range.begin.column == 6);
        CHECK(range.end.column == 8);
    }

    SECTION("Map explicit binding leaf gets pattern range")
    {
        // "def {foo: bar} = x"
        //            ^
        //        col 11-13
        auto result = parse("def {foo: bar} = x");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        auto leaves = find_nodes(program[0], "Destructure_Leaf");
        REQUIRE(leaves.size() == 1);
        auto range = leaves[0]->source_range();
        CHECK(range.begin.column == 11);
        CHECK(range.end.column == 13);
    }

    SECTION("Multiline destructure ranges")
    {
        // "def [\n  a,\n  [b, c]\n] = x"
        auto result = parse("def [\n  a,\n  [b, c]\n] = x");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        auto arrays = find_nodes(program[0], "Destructure_Array");
        REQUIRE(arrays.size() == 2);

        auto outer = arrays[0]->source_range();
        CHECK(outer.begin.line == 1);
        CHECK(outer.begin.column == 5);
        CHECK(outer.end.line == 4);
        CHECK(outer.end.column == 1);

        auto inner = arrays[1]->source_range();
        CHECK(inner.begin.line == 3);
        CHECK(inner.begin.column == 3);
        CHECK(inner.end.line == 3);
        CHECK(inner.end.column == 8);
    }
}
