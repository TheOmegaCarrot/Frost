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

// Find all AST nodes whose node_label starts with the given prefix.
std::vector<const frst::ast::AST_Node*> find_nodes(
    const frst::ast::Statement::Ptr& stmt, std::string_view prefix)
{
    std::vector<const frst::ast::AST_Node*> result;
    for (auto* n : stmt->walk())
        if (n->node_label().starts_with(prefix))
            result.push_back(n);
    return result;
}

// Find all AST nodes whose node_label is *exactly* the given label. Use this
// when a prefix would also match other node types -- e.g., "Match" would
// otherwise match "Match_Binding", "Match_Array", etc.
std::vector<const frst::ast::AST_Node*> find_nodes_exact(
    const frst::ast::Statement::Ptr& stmt, std::string_view label)
{
    std::vector<const frst::ast::AST_Node*> result;
    for (auto* n : stmt->walk())
        if (n->node_label() == label)
            result.push_back(n);
    return result;
}

// Convenience: parse a single statement program and return the first
// statement, asserting that the parse succeeded.
frst::ast::Statement::Ptr parse_one(std::string_view input)
{
    auto result = parse(input);
    REQUIRE(result);
    auto program = std::move(result).value();
    REQUIRE(program.size() == 1);
    return std::move(program[0]);
}

} // namespace

// =============================================================================
// Top-level shape
// =============================================================================

TEST_CASE("Parser Match: minimum case (single arm, no guard)")
{
    frst::Symbol_Table table;
    run("def x = match 1 { _ => 42 }", table);
    CHECK(lookup(table, "x")->get<frst::Int>().value() == 42_f);
}

TEST_CASE("Parser Match: empty arm list throws")
{
    frst::Symbol_Table table;
    CHECK_THROWS(run("def x = match 1 {}", table));
}

TEST_CASE("Parser Match: multiple arms, first matching wins")
{
    frst::Symbol_Table table;
    run("def x = match 2 { 1 => 'one', 2 => 'two', _ => 'other' }", table);
    CHECK(lookup(table, "x")->raw_get<frst::String>() == "two");
}

TEST_CASE("Parser Match: trailing comma after the last arm is allowed")
{
    frst::Symbol_Table table;
    run("def x = match 1 { _ => 42, }", table);
    CHECK(lookup(table, "x")->get<frst::Int>().value() == 42_f);
}

TEST_CASE("Parser Match: arms separated by newlines and commas")
{
    frst::Symbol_Table table;
    run("def x = match 2 {\n"
        "    1 => 'one',\n"
        "    2 => 'two',\n"
        "    _ => 'other',\n"
        "}",
        table);
    CHECK(lookup(table, "x")->raw_get<frst::String>() == "two");
}

TEST_CASE("Parser Match: nested match expressions")
{
    frst::Symbol_Table table;
    run("def x = match (match 1 { 1 => 2, _ => 0 }) { 2 => 'inner', _ => 'no' "
        "}",
        table);
    CHECK(lookup(table, "x")->raw_get<frst::String>() == "inner");
}

TEST_CASE("Parser Match: match in a def binding (expression position)")
{
    frst::Symbol_Table table;
    run("def x = match 5 { n => n + 1 }", table);
    CHECK(lookup(table, "x")->get<frst::Int>().value() == 6_f);
}

TEST_CASE("Parser Match: match passed as a function argument")
{
    frst::Symbol_Table table;
    run("def f = fn x -> x * 2; def y = f(match 3 { n => n })", table);
    CHECK(lookup(table, "y")->get<frst::Int>().value() == 6_f);
}

// =============================================================================
// Pattern dispatch (verifying the right AST nodes are built)
// =============================================================================

TEST_CASE("Parser Match: bare identifier becomes a Match_Binding")
{
    auto stmt = parse_one("def x = match 5 { n => n }");
    auto bindings = find_nodes(stmt, "Match_Binding");
    REQUIRE(bindings.size() == 1);
    CHECK(bindings[0]->node_label() == "Match_Binding(n)");
}

TEST_CASE("Parser Match: `_` becomes a discard binding")
{
    auto stmt = parse_one("def x = match 5 { _ => 42 }");
    auto bindings = find_nodes(stmt, "Match_Binding");
    REQUIRE(bindings.size() == 1);
    CHECK(bindings[0]->node_label() == "Match_Binding(_)");
}

TEST_CASE("Parser Match: identifier with `is TYPE` constraint")
{
    auto stmt = parse_one("def x = match 5 { n is Int => n }");
    auto bindings = find_nodes(stmt, "Match_Binding");
    REQUIRE(bindings.size() == 1);
    CHECK(bindings[0]->node_label() == "Match_Binding(n is Int)");
}

TEST_CASE("Parser Match: discard with `is TYPE` constraint")
{
    auto stmt = parse_one("def x = match 5 { _ is Int => 1 }");
    auto bindings = find_nodes(stmt, "Match_Binding");
    REQUIRE(bindings.size() == 1);
    CHECK(bindings[0]->node_label() == "Match_Binding(_ is Int)");
}

TEST_CASE("Parser Match: every Type_Constraint is parseable")
{
    // Each entry must parse and end up as a Match_Binding with the
    // expected label, not fall back to Match_Value or fail with "unknown
    // type constraint".
    struct Case
    {
        std::string code;
        std::string expected_label;
    };
    const std::vector<Case> cases = {
        {"def x = match 1 { n is Null => 1 }", "Match_Binding(n is Null)"},
        {"def x = match 1 { n is Int => 1 }", "Match_Binding(n is Int)"},
        {"def x = match 1 { n is Float => 1 }", "Match_Binding(n is Float)"},
        {"def x = match 1 { n is Bool => 1 }", "Match_Binding(n is Bool)"},
        {"def x = match 1 { n is String => 1 }", "Match_Binding(n is String)"},
        {"def x = match 1 { n is Array => 1 }", "Match_Binding(n is Array)"},
        {"def x = match 1 { n is Map => 1 }", "Match_Binding(n is Map)"},
        {"def x = match 1 { n is Function => 1 }",
         "Match_Binding(n is Function)"},
        {"def x = match 1 { n is Primitive => 1 }",
         "Match_Binding(n is Primitive)"},
        {"def x = match 1 { n is Numeric => 1 }",
         "Match_Binding(n is Numeric)"},
        {"def x = match 1 { n is Structured => 1 }",
         "Match_Binding(n is Structured)"},
        {"def x = match 1 { n is Nonnull => 1 }",
         "Match_Binding(n is Nonnull)"},
    };

    for (const auto& c : cases)
    {
        DYNAMIC_SECTION(c.expected_label)
        {
            auto stmt = parse_one(c.code);
            auto bindings = find_nodes(stmt, "Match_Binding");
            REQUIRE(bindings.size() == 1);
            CHECK(bindings[0]->node_label() == c.expected_label);
        }
    }
}

TEST_CASE("Parser Match: primitive literal as value pattern")
{
    auto match_int = parse_one("def x = match 42 { 42 => 'int', _ => 'no' }");
    auto values = find_nodes(match_int, "Match_Value");
    REQUIRE(values.size() == 1);

    frst::Symbol_Table table;
    run("def x = match 42 { 42 => 'int', _ => 'no' }", table);
    CHECK(lookup(table, "x")->raw_get<frst::String>() == "int");

    run("def y = match 3.14 { 3.14 => 'flt', _ => 'no' }", table);
    CHECK(lookup(table, "y")->raw_get<frst::String>() == "flt");

    run("def z = match 'hi' { 'hi' => 'str', _ => 'no' }", table);
    CHECK(lookup(table, "z")->raw_get<frst::String>() == "str");

    run("def w = match true { true => 'tru', _ => 'no' }", table);
    CHECK(lookup(table, "w")->raw_get<frst::String>() == "tru");

    run("def n = match null { null => 'nul', _ => 'no' }", table);
    CHECK(lookup(table, "n")->raw_get<frst::String>() == "nul");
}

TEST_CASE("Parser Match: format string as value pattern")
{
    // Format strings evaluate at match time and compare via Frost equality.
    // The interpolated identifier is looked up in the surrounding scope.
    auto stmt =
        parse_one("def x = match 'hi' { $'h${suffix}' => 'yes', _ => 'no' }");
    auto values = find_nodes(stmt, "Match_Value");
    REQUIRE(values.size() == 1);
    auto kids = values[0]->children() | std::ranges::to<std::vector>();
    REQUIRE(kids.size() == 1);
    CHECK(kids[0].node->node_label() == "Format_String(h${suffix})");
    // The inner Format_String must have a real source range.
    auto inner = kids[0].node->source_range();
    CHECK(inner.begin.line != 0);
    CHECK(inner.end.line != 0);

    frst::Symbol_Table table;
    run("def suffix = 'i'\n"
        "def x = match 'hi' { $'h${suffix}' => 'yes', _ => 'no' }",
        table);
    CHECK(lookup(table, "x")->raw_get<frst::String>() == "yes");
}

TEST_CASE("Parser Match: parenthesized expression is a Match_Value")
{
    // The body of the value pattern must NOT be a Literal (it's a Binop).
    auto stmt = parse_one("def x = match 5 { (2 + 3) => 'eq', _ => 'no' }");
    auto values = find_nodes(stmt, "Match_Value");
    REQUIRE(values.size() == 1);

    // Drill into Match_Value's child to confirm it's not a plain Literal.
    auto& value_node = *values[0];
    auto kids = value_node.children() | std::ranges::to<std::vector>();
    REQUIRE(kids.size() == 1);
    CHECK(kids[0].node->node_label() != "Literal");

    frst::Symbol_Table table;
    run("def x = match 5 { (2 + 3) => 'eq', _ => 'no' }", table);
    CHECK(lookup(table, "x")->raw_get<frst::String>() == "eq");
}

TEST_CASE("Parser Match: array pattern with multiple elements")
{
    auto stmt = parse_one("def x = match [1, 2, 3] { [a, b, c] => a + b + c }");
    auto arrays = find_nodes(stmt, "Match_Array");
    REQUIRE(arrays.size() == 1);

    frst::Symbol_Table table;
    run("def x = match [1, 2, 3] { [a, b, c] => a + b + c }", table);
    CHECK(lookup(table, "x")->get<frst::Int>().value() == 6_f);
}

TEST_CASE("Parser Match: empty array pattern matches empty array")
{
    auto stmt = parse_one("def x = match [] { [] => 'empty', _ => 'no' }");
    auto arrays = find_nodes(stmt, "Match_Array");
    REQUIRE(arrays.size() == 1);
    // Empty array pattern has no children
    auto kids = arrays[0]->children() | std::ranges::to<std::vector>();
    CHECK(kids.empty());

    frst::Symbol_Table table;
    run("def x = match [] { [] => 'empty', _ => 'no' }", table);
    CHECK(lookup(table, "x")->raw_get<frst::String>() == "empty");
}

TEST_CASE("Parser Match: array pattern with named rest")
{
    auto stmt = parse_one("def x = match [1, 2, 3] { [a, ...rest] => rest }");
    auto arrays = find_nodes(stmt, "Match_Array");
    REQUIRE(arrays.size() == 1);
    CHECK(arrays[0]->node_label() == "Match_Array(...rest)");

    frst::Symbol_Table table;
    run("def x = match [1, 2, 3] { [a, ...rest] => rest }", table);
    auto& result = lookup(table, "x")->raw_get<frst::Array>();
    REQUIRE(result.size() == 2);
    CHECK(result[0]->get<frst::Int>().value() == 2_f);
    CHECK(result[1]->get<frst::Int>().value() == 3_f);
}

TEST_CASE("Parser Match: array pattern with discard rest")
{
    auto stmt = parse_one("def x = match [1, 2, 3] { [a, ..._] => a, _ => 0 }");
    auto arrays = find_nodes(stmt, "Match_Array");
    REQUIRE(arrays.size() == 1);
    CHECK(arrays[0]->node_label() == "Match_Array(..._)");

    frst::Symbol_Table table;
    run("def x = match [1, 2, 3] { [a, ..._] => a, _ => 0 }", table);
    CHECK(lookup(table, "x")->get<frst::Int>().value() == 1_f);
}

TEST_CASE("Parser Match: rest-only array patterns")
{
    auto named = parse_one("def x = match [1, 2] { [...all] => all }");
    auto named_arrays = find_nodes(named, "Match_Array");
    REQUIRE(named_arrays.size() == 1);
    CHECK(named_arrays[0]->node_label() == "Match_Array(...all)");

    auto discard = parse_one("def x = match [1, 2] { [..._] => 'any' }");
    auto discard_arrays = find_nodes(discard, "Match_Array");
    REQUIRE(discard_arrays.size() == 1);
    CHECK(discard_arrays[0]->node_label() == "Match_Array(..._)");

    frst::Symbol_Table table;
    run("def x = match [1, 2] { [...all] => all }", table);
    CHECK(lookup(table, "x")->raw_get<frst::Array>().size() == 2);
}

TEST_CASE("Parser Match: explicit map entry `key: pattern`")
{
    auto stmt =
        parse_one("def x = match {a: 1, b: 2} { {a: x, b: y} => x + y }");
    auto maps = find_nodes(stmt, "Match_Map");
    REQUIRE(maps.size() == 1);

    frst::Symbol_Table table;
    run("def x = match {a: 1, b: 2} { {a: x, b: y} => x + y }", table);
    CHECK(lookup(table, "x")->get<frst::Int>().value() == 3_f);
}

TEST_CASE("Parser Match: computed-key map entry `[expr]: pattern`")
{
    frst::Symbol_Table table;
    run("def k = 'name'\n"
        "def x = match {name: 'Alice'} { {[k]: n} => n, _ => 'none' }",
        table);
    CHECK(lookup(table, "x")->raw_get<frst::String>() == "Alice");
}

TEST_CASE("Parser Match: shorthand `{key}` desugars to `{key: key}`")
{
    auto stmt = parse_one("def x = match {a: 1} { {a} => a }");
    auto bindings = find_nodes(stmt, "Match_Binding");
    REQUIRE(bindings.size() == 1);
    CHECK(bindings[0]->node_label() == "Match_Binding(a)");

    frst::Symbol_Table table;
    run("def x = match {a: 1} { {a} => a }", table);
    CHECK(lookup(table, "x")->get<frst::Int>().value() == 1_f);
}

TEST_CASE("Parser Match: shorthand with constraint `{key is Int}`")
{
    auto stmt = parse_one("def x = match {a: 1} { {a is Int} => a, _ => 0 }");
    auto bindings = find_nodes(stmt, "Match_Binding");
    // One for `a is Int`, one for the catch-all `_`.
    REQUIRE(bindings.size() == 2);
    CHECK(bindings[0]->node_label() == "Match_Binding(a is Int)");
    CHECK(bindings[1]->node_label() == "Match_Binding(_)");

    frst::Symbol_Table table;
    run("def x = match {a: 1} { {a is Int} => a, _ => 0 }", table);
    CHECK(lookup(table, "x")->get<frst::Int>().value() == 1_f);

    // Mismatched constraint -> falls through.
    run("def y = match {a: 'hi'} { {a is Int} => a, _ => 'no' }", table);
    CHECK(lookup(table, "y")->raw_get<frst::String>() == "no");
}

TEST_CASE("Parser Match: empty map pattern matches any map")
{
    auto stmt = parse_one("def x = match {a: 1} { {} => 'any', _ => 'no' }");
    auto maps = find_nodes(stmt, "Match_Map");
    REQUIRE(maps.size() == 1);
    auto kids = maps[0]->children() | std::ranges::to<std::vector>();
    CHECK(kids.empty());

    frst::Symbol_Table table;
    run("def x = match {a: 1} { {} => 'any', _ => 'no' }", table);
    CHECK(lookup(table, "x")->raw_get<frst::String>() == "any");
}

TEST_CASE("Parser Match: mixed map entry forms in one pattern")
{
    frst::Symbol_Table table;
    run("def k = 'computed'\n"
        "def m = {a: 1, b: 2, computed: 3, c: 4}\n"
        "def x = match m { {a, b: bb, [k]: cc, c is Int} => a + bb + cc + c }",
        table);
    CHECK(lookup(table, "x")->get<frst::Int>().value() == 10_f);
}

// =============================================================================
// Nesting
// =============================================================================

TEST_CASE("Parser Match: array inside array")
{
    frst::Symbol_Table table;
    run("def x = match [1, [2, 3]] { [a, [b, c]] => a + b + c }", table);
    CHECK(lookup(table, "x")->get<frst::Int>().value() == 6_f);
}

TEST_CASE("Parser Match: array inside map")
{
    frst::Symbol_Table table;
    run("def x = match {pair: [1, 2]} { {pair: [a, b]} => a + b }", table);
    CHECK(lookup(table, "x")->get<frst::Int>().value() == 3_f);
}

TEST_CASE("Parser Match: map inside array")
{
    frst::Symbol_Table table;
    run("def x = match [{a: 1}, {a: 2}] { [{a: x}, {a: y}] => x + y }", table);
    CHECK(lookup(table, "x")->get<frst::Int>().value() == 3_f);
}

TEST_CASE("Parser Match: type constraints on array elements")
{
    auto stmt = parse_one("def x = match [1, 'hi'] { [a is Int, b is String] "
                          "=> 'ok', _ => 'no' }");
    auto bindings = find_nodes(stmt, "Match_Binding");
    // Two for the array elements, one for the catch-all `_`.
    REQUIRE(bindings.size() == 3);
    CHECK(bindings[0]->node_label() == "Match_Binding(a is Int)");
    CHECK(bindings[1]->node_label() == "Match_Binding(b is String)");
    CHECK(bindings[2]->node_label() == "Match_Binding(_)");

    frst::Symbol_Table table;
    run("def x = match [1, 'hi'] { [a is Int, b is String] => 'ok', _ => 'no' "
        "}",
        table);
    CHECK(lookup(table, "x")->raw_get<frst::String>() == "ok");
}

// =============================================================================
// Guards
// =============================================================================

TEST_CASE("Parser Match: guard with `if:` syntax")
{
    frst::Symbol_Table table;
    run("def x = match 5 { n if: n > 3 => 'big', _ => 'small' }", table);
    CHECK(lookup(table, "x")->raw_get<frst::String>() == "big");

    run("def y = match 1 { n if: n > 3 => 'big', _ => 'small' }", table);
    CHECK(lookup(table, "y")->raw_get<frst::String>() == "small");
}

TEST_CASE("Parser Match: guard expression can be complex")
{
    frst::Symbol_Table table;
    run("def f = fn x -> x > 0\n"
        "def x = match 5 { n if: f(n) and n < 10 => 'mid', _ => 'no' }",
        table);
    CHECK(lookup(table, "x")->raw_get<frst::String>() == "mid");
}

TEST_CASE("Parser Match: arms without guards are still well-formed")
{
    // Mix guarded and unguarded arms in the same match.
    frst::Symbol_Table table;
    run("def x = match 5 { n if: n > 10 => 'big', n => 'small' }", table);
    CHECK(lookup(table, "x")->raw_get<frst::String>() == "small");
}

// =============================================================================
// Disambiguation rules
// =============================================================================

TEST_CASE("Parser Match: bare identifier is a binding, not a value pattern")
{
    auto stmt = parse_one("def x = match 5 { n => n }");
    auto bindings = find_nodes(stmt, "Match_Binding");
    auto values = find_nodes(stmt, "Match_Value");
    CHECK(bindings.size() == 1);
    CHECK(values.empty());
}

TEST_CASE("Parser Match: parenthesized identifier is a value pattern")
{
    // This is the user's canonical disambiguation example: `(x)` looks up
    // an existing `x` and compares against it; bare `x` would be a new
    // binding.
    auto stmt = parse_one("def y = match 7 { (x) => 'eq', _ => 'no' }");
    auto bindings = find_nodes(stmt, "Match_Binding");
    auto values = find_nodes(stmt, "Match_Value");
    // Only one Match_Binding (the `_` in the catch-all arm).
    REQUIRE(bindings.size() == 1);
    CHECK(bindings[0]->node_label() == "Match_Binding(_)");
    REQUIRE(values.size() == 1);

    frst::Symbol_Table table;
    run("def x = 7\n"
        "def y = match 7 { (x) => 'eq', _ => 'no' }",
        table);
    CHECK(lookup(table, "y")->raw_get<frst::String>() == "eq");
}

TEST_CASE("Parser Match: literal is always a value pattern")
{
    auto stmt = parse_one("def x = match 5 { 5 => 'lit', _ => 'no' }");
    auto values = find_nodes(stmt, "Match_Value");
    REQUIRE(values.size() == 1);
}

// =============================================================================
// Whitespace tolerance
// =============================================================================

TEST_CASE("Parser Match: newline before `=>`")
{
    frst::Symbol_Table table;
    run("def x = match 1 {\n"
        "    n\n"
        "        => n + 1\n"
        "}",
        table);
    CHECK(lookup(table, "x")->get<frst::Int>().value() == 2_f);
}

TEST_CASE("Parser Match: newline after `=>`")
{
    frst::Symbol_Table table;
    run("def x = match 1 {\n"
        "    n =>\n"
        "        n + 1\n"
        "}",
        table);
    CHECK(lookup(table, "x")->get<frst::Int>().value() == 2_f);
}

TEST_CASE("Parser Match: newlines inside array pattern")
{
    frst::Symbol_Table table;
    run("def x = match [1, 2, 3] {\n"
        "    [\n"
        "        a,\n"
        "        b,\n"
        "        c\n"
        "    ] => a + b + c\n"
        "}",
        table);
    CHECK(lookup(table, "x")->get<frst::Int>().value() == 6_f);
}

TEST_CASE("Parser Match: newlines inside map pattern")
{
    frst::Symbol_Table table;
    run("def x = match {a: 1, b: 2} {\n"
        "    {\n"
        "        a: aa,\n"
        "        b: bb,\n"
        "    } => aa + bb\n"
        "}",
        table);
    CHECK(lookup(table, "x")->get<frst::Int>().value() == 3_f);
}

TEST_CASE("Parser Match: multi-line result expression")
{
    frst::Symbol_Table table;
    run("def x = match 5 {\n"
        "    n => do {\n"
        "        def doubled = n * 2\n"
        "        doubled + 1\n"
        "    }\n"
        "}",
        table);
    CHECK(lookup(table, "x")->get<frst::Int>().value() == 11_f);
}

// =============================================================================
// Source ranges
// =============================================================================

TEST_CASE("Parser Match Source Ranges: Match_Binding range")
{
    // "def x = match 1 { foo => 1 }"
    //                    ^^^
    //                col 19-21
    auto stmt = parse_one("def x = match 1 { foo => 1 }");
    auto bindings = find_nodes(stmt, "Match_Binding");
    REQUIRE(bindings.size() == 1);
    auto range = bindings[0]->source_range();
    CHECK(range.begin.line == 1);
    CHECK(range.begin.column == 19);
    CHECK(range.end.column == 21);
}

TEST_CASE("Parser Match Source Ranges: array pattern spans brackets")
{
    // "def x = match [1, 2] { [a, b] => 0 }"
    //                         ^    ^
    //                     col 24  29
    auto stmt = parse_one("def x = match [1, 2] { [a, b] => 0 }");
    auto arrays = find_nodes(stmt, "Match_Array");
    REQUIRE(arrays.size() == 1);
    auto range = arrays[0]->source_range();
    CHECK(range.begin.column == 24);
    CHECK(range.end.column == 29);
}

TEST_CASE("Parser Match Source Ranges: map pattern spans braces")
{
    // "def x = match {a: 1} { {a} => 0 }"
    //                         ^^^
    //                     col 24-26
    auto stmt = parse_one("def x = match {a: 1} { {a} => 0 }");
    auto maps = find_nodes(stmt, "Match_Map");
    REQUIRE(maps.size() == 1);
    auto range = maps[0]->source_range();
    CHECK(range.begin.column == 24);
    CHECK(range.end.column == 26);
}

TEST_CASE("Parser Match Source Ranges: nested arrays have independent ranges")
{
    // "def x = match [[1]] { [[a]] => 0 }"
    //                        ^   ^
    //                    col 23  27
    //                         ^^^
    //                     col 24-26
    auto stmt = parse_one("def x = match [[1]] { [[a]] => 0 }");
    auto arrays = find_nodes(stmt, "Match_Array");
    REQUIRE(arrays.size() == 2);

    // Outer array
    auto outer = arrays[0]->source_range();
    CHECK(outer.begin.column == 23);
    CHECK(outer.end.column == 27);

    // Inner array
    auto inner = arrays[1]->source_range();
    CHECK(inner.begin.column == 24);
    CHECK(inner.end.column == 26);
}

// =============================================================================
// Parse errors
// =============================================================================

TEST_CASE("Parser Match Errors: unknown type constraint")
{
    auto result = parse("def x = match 1 { n is FooBar => n, _ => 0 }");
    CHECK_FALSE(result);
}

TEST_CASE("Parser Match Errors: missing result expression")
{
    auto result = parse("def x = match 1 { n => }");
    CHECK_FALSE(result);
}

TEST_CASE("Parser Match Errors: missing arrow")
{
    auto result = parse("def x = match 1 { n 1 }");
    CHECK_FALSE(result);
}

TEST_CASE("Parser Match Errors: missing comma between arms")
{
    auto result = parse("def x = match 1 { 1 => 'a' 2 => 'b' }");
    CHECK_FALSE(result);
}

TEST_CASE("Parser Match Errors: missing colon in guard")
{
    auto result = parse("def x = match 1 { n if n > 0 => n }");
    CHECK_FALSE(result);
}

TEST_CASE("Parser Match Errors: missing braces")
{
    auto result = parse("def x = match 1 n => 1");
    CHECK_FALSE(result);
}

TEST_CASE("Parser Match Errors: empty arm (missing pattern)")
{
    auto result = parse("def x = match 1 { => 1 }");
    CHECK_FALSE(result);
}

TEST_CASE("Parser Match Errors: `is` after a value pattern is rejected")
{
    // `is TYPE` only attaches to a Match_Binding, not a Match_Value.
    // `(x) is Int` should be a parse error.
    auto result = parse("def x = match 1 { (x) is Int => 1, _ => 0 }");
    CHECK_FALSE(result);
}

TEST_CASE("Parser Match Errors: missing target expression")
{
    auto result = parse("def x = match { n => n }");
    CHECK_FALSE(result);
}

TEST_CASE("Parser Match Errors: case-sensitive type constraint name")
{
    // Type constraint names are case-sensitive: `int` is NOT `Int`.
    auto result = parse("def x = match 1 { n is int => n, _ => 0 }");
    CHECK_FALSE(result);
}

TEST_CASE("Parser Match Errors: keyword as binding name is rejected")
{
    // `match` and `is` are global keywords; they can't be used as binding
    // names anywhere, including in match patterns.
    CHECK_FALSE(parse("def x = match 5 { match => 1 }"));
    CHECK_FALSE(parse("def x = match 5 { is => 1 }"));
}

TEST_CASE("Parser Match Errors: bare negative literal is not a value pattern")
{
    // The match dispatcher peeks at the first character to choose a branch.
    // `-` doesn't begin any pattern (the unary minus operator is not a
    // literal), so `-1 => ...` must fail to parse. The user must wrap it:
    // `(-1) => ...`.
    auto result = parse("def x = match -1 { -1 => 'neg', _ => 'pos' }");
    CHECK_FALSE(result);
}

TEST_CASE("Parser Match: trailing comma in array pattern is accepted")
{
    frst::Symbol_Table table;
    run("def x = match [1, 2] { [a, b,] => a + b, _ => 0 }", table);
    CHECK(lookup(table, "x")->get<frst::Int>().value() == 3_f);
}

TEST_CASE("Parser Match: trailing comma in map pattern is accepted")
{
    frst::Symbol_Table table;
    run("def x = match {a: 1, b: 2} { {a, b,} => a + b, _ => 0 }", table);
    CHECK(lookup(table, "x")->get<frst::Int>().value() == 3_f);
}

TEST_CASE(
    "Parser Match Errors: trailing comma after array rest clause is rejected")
{
    // Trailing commas are only allowed where another element could
    // syntactically follow. The rest clause is the syntactic terminator
    // of the array, so nothing can follow it -- including a trailing comma.
    CHECK_FALSE(parse("def x = match [1, 2] { [a, ...rest,] => 0 }"));
    CHECK_FALSE(parse("def x = match [1, 2] { [...rest,] => 0 }"));
}

// =============================================================================
// Position / context: match in various expression positions
// =============================================================================

TEST_CASE("Parser Match: match expression as a top-level statement")
{
    // A bare `match ... { ... }` at the top level parses as an expression
    // statement -- the result value is simply discarded.
    auto stmt = parse_one("match 5 { _ => 1 }");
    CHECK(stmt->node_label() == "Match");
}

TEST_CASE("Parser Match: match inside an array literal")
{
    frst::Symbol_Table table;
    run("def x = [match 5 { _ => 'a' }, match 6 { _ => 'b' }]", table);
    auto& arr = lookup(table, "x")->raw_get<frst::Array>();
    REQUIRE(arr.size() == 2);
    CHECK(arr[0]->raw_get<frst::String>() == "a");
    CHECK(arr[1]->raw_get<frst::String>() == "b");
}

TEST_CASE("Parser Match: match as a map value")
{
    frst::Symbol_Table table;
    run("def x = {result: match 5 { n => n + 1 }}", table);
    auto& m = lookup(table, "x")->raw_get<frst::Map>();
    auto it = m.find(frst::Value::create("result"s));
    REQUIRE(it != m.end());
    CHECK(it->second->get<frst::Int>().value() == 6_f);
}

TEST_CASE("Parser Match: match followed by binary operator")
{
    // `match ... { ... } + 2` should parse as `Binop(Match, +, 2)`. The
    // match expression is at primary-expression precedence so it composes
    // freely with the operator tower.
    auto stmt = parse_one("def x = match 5 { _ => 1 } + 2");
    auto matches = find_nodes_exact(stmt, "Match");
    REQUIRE(matches.size() == 1);
    auto binops = find_nodes(stmt, "Binary(+)");
    REQUIRE(binops.size() == 1);

    frst::Symbol_Table table;
    run("def x = match 5 { _ => 1 } + 2", table);
    CHECK(lookup(table, "x")->get<frst::Int>().value() == 3_f);
}

TEST_CASE("Parser Match: match as the result of another match arm")
{
    frst::Symbol_Table table;
    run("def x = match 1 {\n"
        "    n => match n {\n"
        "        1 => 'one',\n"
        "        _ => 'other'\n"
        "    }\n"
        "}",
        table);
    CHECK(lookup(table, "x")->raw_get<frst::String>() == "one");
}

// =============================================================================
// Pattern dispatch: extra cases
// =============================================================================

TEST_CASE("Parser Match: parenthesized negative literal as value pattern")
{
    // Bare `-1` is not a valid pattern, but `(-1)` is -- it goes through
    // the parenthesized branch and the inner expression is a unary `-`
    // applied to a literal.
    auto stmt = parse_one("def x = match -1 { (-1) => 'neg', _ => 'pos' }");
    auto values = find_nodes(stmt, "Match_Value");
    REQUIRE(values.size() == 1);

    frst::Symbol_Table table;
    run("def x = match -1 { (-1) => 'neg', _ => 'pos' }", table);
    CHECK(lookup(table, "x")->raw_get<frst::String>() == "neg");
}

TEST_CASE("Parser Match: multiple `_` in the same array pattern")
{
    // P2169-style: discards do not conflict with each other within the
    // same scope. Three independent discards in one array pattern.
    auto stmt =
        parse_one("def x = match [1, 2, 3] { [_, _, _] => 'ok', _ => 'no' }");
    auto bindings = find_nodes(stmt, "Match_Binding");
    // Three inside the array, one for the catch-all.
    REQUIRE(bindings.size() == 4);
    for (std::size_t i = 0; i < 3; ++i)
        CHECK(bindings[i]->node_label() == "Match_Binding(_)");

    frst::Symbol_Table table;
    run("def x = match [1, 2, 3] { [_, _, _] => 'ok', _ => 'no' }", table);
    CHECK(lookup(table, "x")->raw_get<frst::String>() == "ok");
}

// =============================================================================
// Disambiguation: bare `_` is a discard binding
// =============================================================================

TEST_CASE("Parser Match: bare `_` is a Match_Binding, not a Match_Value")
{
    // `_` is an identifier (per the lexer), so it goes through the binding
    // branch -- not the literal branch -- and produces a discard binding.
    auto stmt = parse_one("def x = match 5 { _ => 'any' }");
    auto bindings = find_nodes(stmt, "Match_Binding");
    auto values = find_nodes(stmt, "Match_Value");
    REQUIRE(bindings.size() == 1);
    CHECK(bindings[0]->node_label() == "Match_Binding(_)");
    CHECK(values.empty());
}

// =============================================================================
// Source ranges: extra cases
// =============================================================================

TEST_CASE(
    "Parser Match Source Ranges: top-level Match node spans `match` to `}`")
{
    // "def x = match 1 { _ => 1 }"
    //         ^                ^
    //     col 9             col 26
    auto stmt = parse_one("def x = match 1 { _ => 1 }");
    auto matches = find_nodes_exact(stmt, "Match");
    REQUIRE(matches.size() == 1);
    auto range = matches[0]->source_range();
    CHECK(range.begin.line == 1);
    CHECK(range.begin.column == 9);
    CHECK(range.end.line == 1);
    CHECK(range.end.column == 26);
}

TEST_CASE("Parser Match Source Ranges: Match_Binding with type constraint "
          "covers `name is TYPE`")
{
    // "def x = match 1 { n is Int => 1, _ => 0 }"
    //                    ^      ^
    //                col 19   col 26
    auto stmt = parse_one("def x = match 1 { n is Int => 1, _ => 0 }");
    auto bindings = find_nodes(stmt, "Match_Binding");
    REQUIRE(bindings.size() == 2);
    auto range = bindings[0]->source_range();
    CHECK(range.begin.column == 19);
    CHECK(range.end.column == 26);
}

TEST_CASE("Parser Match Source Ranges: Match_Value spans the literal token")
{
    // "def x = match 5 { 42 => 'a', _ => 'b' }"
    //                    ^^
    //                col 19-20
    auto stmt = parse_one("def x = match 5 { 42 => 'a', _ => 'b' }");
    auto values = find_nodes(stmt, "Match_Value");
    REQUIRE(values.size() == 1);
    auto range = values[0]->source_range();
    CHECK(range.begin.column == 19);
    CHECK(range.end.column == 20);

    // The inner expression must NOT be no_range -- the literal-form value
    // pattern is responsible for assigning a range to its child.
    auto kids = values[0]->children() | std::ranges::to<std::vector>();
    REQUIRE(kids.size() == 1);
    auto inner = kids[0].node->source_range();
    CHECK(inner.begin.line != 0);
    CHECK(inner.end.line != 0);
}

TEST_CASE("Parser Match Source Ranges: Match_Value spans parenthesized "
          "expression including parens")
{
    // "def x = match 5 { (2 + 3) => 'a', _ => 'b' }"
    //                    ^     ^
    //                col 19  col 25
    auto stmt = parse_one("def x = match 5 { (2 + 3) => 'a', _ => 'b' }");
    auto values = find_nodes(stmt, "Match_Value");
    REQUIRE(values.size() == 1);
    auto range = values[0]->source_range();
    CHECK(range.begin.column == 19);
    CHECK(range.end.column == 25);
}

TEST_CASE("Parser Match Source Ranges: multi-line match spans across lines")
{
    auto stmt = parse_one("def x = match 1 {\n"
                          "    n => n + 1\n"
                          "}");
    auto matches = find_nodes_exact(stmt, "Match");
    REQUIRE(matches.size() == 1);
    auto range = matches[0]->source_range();
    CHECK(range.begin.line == 1);
    CHECK(range.begin.column == 9);
    CHECK(range.end.line == 3);
    CHECK(range.end.column == 1);
}

// =============================================================================
// Whitespace / lexical: extra cases
// =============================================================================

TEST_CASE("Parser Match: comment inside arm list")
{
    frst::Symbol_Table table;
    run("def x = match 5 {\n"
        "    # this arm catches everything\n"
        "    n => n + 1\n"
        "}",
        table);
    CHECK(lookup(table, "x")->get<frst::Int>().value() == 6_f);
}

TEST_CASE("Parser Match: newline immediately after opening brace")
{
    frst::Symbol_Table table;
    run("def x = match 5 {\n"
        "_ => 42\n"
        "}",
        table);
    CHECK(lookup(table, "x")->get<frst::Int>().value() == 42_f);
}

// =============================================================================
// Stress: weird-but-valid combinations all evaluated end-to-end
// =============================================================================

TEST_CASE("Parser Match: stress test")
{
    // Each section throws a chunk of unusual-but-valid match syntax at the
    // parser, then evaluates it and checks the resulting binding. The goal
    // is to catch cross-feature interactions and formatting edge cases that
    // the focused tests don't exercise individually.
    //
    // The parser tests run without the prelude or any builtins loaded, so
    // these chunks must rely only on language constructs: literals,
    // arithmetic, comparison, lambdas, destructuring, indexing, `do`/`if`
    // blocks, the comprehension forms (`map ... with`, `filter ... with`,
    // `reduce ... with`), and match itself.

    SECTION("Deeply nested arrays and maps with mixed pattern kinds")
    {
        frst::Symbol_Table table;
        run(R"(
            def data = {
                user: {
                    name: 'alice',
                    tags: ['admin', 'staff', 'oncall'],
                    meta: {age: 30, active: true}
                }
            }
            def x = match data {
                {
                    user: {
                        name: n is String,
                        tags: [first, second, third],
                        meta: {age: a is Int, active: true}
                    }
                } => [n, first, second, third, a],
                _ => null
            }
        )",
            table);
        auto& arr = lookup(table, "x")->raw_get<frst::Array>();
        REQUIRE(arr.size() == 5);
        CHECK(arr[0]->raw_get<frst::String>() == "alice");
        CHECK(arr[1]->raw_get<frst::String>() == "admin");
        CHECK(arr[2]->raw_get<frst::String>() == "staff");
        CHECK(arr[3]->raw_get<frst::String>() == "oncall");
        CHECK(arr[4]->get<frst::Int>().value() == 30_f);
    }

    SECTION("Many arms, first matching wins, with weird whitespace")
    {
        // Order matters: more specific arms first. `_ is Map` is placed
        // before any `{...}` arm because an empty/loose map pattern would
        // otherwise consume populated maps too.
        frst::Symbol_Table table;
        run(R"(
            def classify = fn v -> match v {
                null                  => 'nil',
                true                  => 'tru',
                false                 => 'fls',
                0                     => 'zero',
                n is Int    if: n < 0 => 'neg-int',
                n is Int    if: n > 0 => 'pos-int',
                f is Float  if: f < 0 => 'neg-flt',
                f is Float            => 'flt',
                ''                    => 'empty-str',
                s is String           => 'str',
                []                    => 'empty-arr',
                [_]                   => 'one-arr',
                [_, _, ..._]          => 'many-arr',
                _ is Map              => 'map',
                _                     => 'other'
            }
            def inputs = [null, true, false, 0, -3, 7, -1.5, 2.5, '', 'hi',
                          [], [1], [1, 2, 3, 4], {}, {a: 1}]
            def results = map inputs with classify
        )",
            table);
        auto& r = lookup(table, "results")->raw_get<frst::Array>();
        std::vector<std::string> expected = {
            "nil",       "tru",     "fls",      "zero",      "neg-int",
            "pos-int",   "neg-flt", "flt",      "empty-str", "str",
            "empty-arr", "one-arr", "many-arr", "map",       "map"};
        REQUIRE(r.size() == expected.size());
        for (std::size_t i = 0; i < expected.size(); ++i)
            CHECK(r[i]->raw_get<frst::String>() == expected[i]);
    }

    SECTION("Trailing commas everywhere they're allowed")
    {
        frst::Symbol_Table table;
        run(R"(
            def x = match [1, 2, 3] {
                [a, b, c,] => a + b + c,
                _ => 0,
            }
            def y = match {a: 1, b: 2,} {
                {a, b,} => a + b,
                _ => 0,
            }
            def z = match [1, 2, [3, 4,], {k: 5,},] {
                [_, _, [_, last,], {k: kv,},] => last + kv,
                _ => 0,
            }
        )",
            table);
        CHECK(lookup(table, "x")->get<frst::Int>().value() == 6_f);
        CHECK(lookup(table, "y")->get<frst::Int>().value() == 3_f);
        CHECK(lookup(table, "z")->get<frst::Int>().value() == 9_f);
    }

    SECTION("Match in every conceivable expression position")
    {
        frst::Symbol_Table table;
        run(R"(
            def in_array = [
                match 1 { _ => 'a' },
                match 2 { _ => 'b' }
            ]
            def in_map = {
                first: match 1 { n => n + 1 },
                second: match 'x' { 'x' => 'X', _ => '?' }
            }
            def index_into_match = (match [10, 20, 30] { [...all] => all })[1]
            def nested = match (match 2 { 2 => 'two', _ => 'no' }) {
                'two' => 2,
                _ => 0
            }
            def in_arm_result = match 1 {
                n => match n {
                    1 => match 'inner' {
                        'inner' => 'deep',
                        _ => '!'
                    },
                    _ => '?'
                }
            }
            def with_op = match 5 { _ => 10 } * 2 + 1
            def in_lambda = (fn x -> match x { n => n + 100 })(7)
            def in_pipeline = 5 @ (fn n -> match n { _ => n * n })()
        )",
            table);
        CHECK(lookup(table, "in_array")->raw_get<frst::Array>().size() == 2);
        auto& m = lookup(table, "in_map")->raw_get<frst::Map>();
        CHECK(m.find(frst::Value::create("first"s))
                  ->second->get<frst::Int>()
                  .value()
              == 2_f);
        CHECK(m.find(frst::Value::create("second"s))
                  ->second->raw_get<frst::String>()
              == "X");
        CHECK(lookup(table, "index_into_match")->get<frst::Int>().value()
              == 20_f);
        CHECK(lookup(table, "nested")->get<frst::Int>().value() == 2_f);
        CHECK(lookup(table, "in_arm_result")->raw_get<frst::String>()
              == "deep");
        CHECK(lookup(table, "with_op")->get<frst::Int>().value() == 21_f);
        CHECK(lookup(table, "in_lambda")->get<frst::Int>().value() == 107_f);
        CHECK(lookup(table, "in_pipeline")->get<frst::Int>().value() == 25_f);
    }

    SECTION(
        "Format strings, computed keys, shorthand, and constraints together")
    {
        frst::Symbol_Table table;
        run(R"(
            def k1 = 'dynamic'
            def data = {
                name: 'world',
                dynamic: 42,
                count: 3,
                items: ['a', 'b', 'c']
            }
            def x = match data {
                {
                    name: n is String,
                    [k1]: v is Int,
                    count is Int,
                    items: [_, mid, _]
                } if: v > count => $'${n}-${mid}',
                _ => 'no'
            }
        )",
            table);
        CHECK(lookup(table, "x")->raw_get<frst::String>() == "world-b");
    }

    SECTION("Format string as a value pattern, with surrounding lookup")
    {
        frst::Symbol_Table table;
        run(R"(
            def prefix = 'hello'
            def name = 'world'
            def greeting = 'hello, world'
            def x = match greeting {
                $'${prefix}, ${name}' => 'matched',
                _ => 'no'
            }
        )",
            table);
        CHECK(lookup(table, "x")->raw_get<frst::String>() == "matched");
    }

    SECTION("Heavy guard expressions over multiple bindings")
    {
        frst::Symbol_Table table;
        run(R"(
            def classify = fn pt -> match pt {
                [x, y] if: x == 0 and y == 0 => 'origin',
                [x, y] if: x == 0            => 'on-y',
                [x, y] if: y == 0            => 'on-x',
                [x, y] if: x > 0 and y > 0   => 'q1',
                [x, y] if: x < 0 and y > 0   => 'q2',
                [x, y] if: x < 0 and y < 0   => 'q3',
                [x, y] if: x > 0 and y < 0   => 'q4',
                _                            => 'unknown'
            }
            def results = map [
                [0, 0], [0, 5], [3, 0],
                [1, 1], [-1, 1], [-1, -1], [1, -1]
            ] with classify
        )",
            table);
        auto& r = lookup(table, "results")->raw_get<frst::Array>();
        std::vector<std::string> expected = {"origin", "on-y", "on-x", "q1",
                                             "q2",     "q3",   "q4"};
        REQUIRE(r.size() == expected.size());
        for (std::size_t i = 0; i < expected.size(); ++i)
            CHECK(r[i]->raw_get<frst::String>() == expected[i]);
    }

    SECTION("Multi-line do-block results, comments scattered throughout")
    {
        frst::Symbol_Table table;
        run(R"(
            def x = match [1, 2, 3] {
                # first arm: empty array (won't match)
                [] => 0,

                # second arm: destructure head and the next two elements
                # via a fully-spelled-out array pattern
                [head, second, third] => do {
                    # build a number from the parts
                    def base = head * 100
                    # mid contributes the tens place
                    def mid = second * 10
                    base + mid + third
                },

                # catch-all
                _ => -1
            }
        )",
            table);
        CHECK(lookup(table, "x")->get<frst::Int>().value() == 123_f);
    }

    SECTION("Bindings shadowing outer scope, plus parenthesized lookup")
    {
        frst::Symbol_Table table;
        run(R"(
            def target = 7
            def shadow_test = match 99 {
                target => target + 1
            }
            def lookup_test = match 7 {
                (target) => 'matched-existing',
                _ => 'no'
            }
        )",
            table);
        CHECK(lookup(table, "shadow_test")->get<frst::Int>().value() == 100_f);
        CHECK(lookup(table, "lookup_test")->raw_get<frst::String>()
              == "matched-existing");
    }

    SECTION("Empty containers in nested positions")
    {
        frst::Symbol_Table table;
        run(R"(
            def x = match [[], {}, [1]] {
                [[], _ is Map, [_]] => 'ok',
                _ => 'no'
            }
            def y = match {a: [], b: {x: 1}} {
                {a: [], b: _ is Map} => 'ok',
                _ => 'no'
            }
            def z = match [] {
                [] => 'empty',
                _ => 'no'
            }
        )",
            table);
        CHECK(lookup(table, "x")->raw_get<frst::String>() == "ok");
        CHECK(lookup(table, "y")->raw_get<frst::String>() == "ok");
        CHECK(lookup(table, "z")->raw_get<frst::String>() == "empty");
    }

    SECTION("Every Type_Constraint reachable in nested positions")
    {
        frst::Symbol_Table table;
        run(R"(
            def all_types = [null, 1, 1.5, true, 'hi', [1], {a: 1},
                             fn x -> x]
            def labels = map all_types with fn v -> match v {
                _ is Null      => 'null',
                _ is Int       => 'int',
                _ is Float     => 'float',
                _ is Bool      => 'bool',
                _ is String    => 'string',
                _ is Array     => 'array',
                _ is Map       => 'map',
                _ is Function  => 'function',
                _              => 'unknown'
            }
            # Composite constraints: every entry should resolve to *some*
            # label, and no arm should be unreachable. Numeric is checked
            # before Primitive so ints/floats land there first.
            def composite = map all_types with fn v -> match v {
                _ is Null       => 'null',
                _ is Numeric    => 'numeric',
                _ is Primitive  => 'primitive',
                _ is Structured => 'structured',
                _ is Function   => 'function',
                _               => 'other'
            }
        )",
            table);
        auto& l = lookup(table, "labels")->raw_get<frst::Array>();
        std::vector<std::string> expected_labels = {"null", "int",     "float",
                                                    "bool", "string",  "array",
                                                    "map",  "function"};
        REQUIRE(l.size() == expected_labels.size());
        for (std::size_t i = 0; i < expected_labels.size(); ++i)
            CHECK(l[i]->raw_get<frst::String>() == expected_labels[i]);

        auto& c = lookup(table, "composite")->raw_get<frst::Array>();
        std::vector<std::string> expected_composite = {
            "null",      "numeric",    "numeric",    "primitive",
            "primitive", "structured", "structured", "function"};
        REQUIRE(c.size() == expected_composite.size());
        for (std::size_t i = 0; i < expected_composite.size(); ++i)
            CHECK(c[i]->raw_get<frst::String>() == expected_composite[i]);
    }

    SECTION("Long arm list with many disambiguation forms in one match")
    {
        frst::Symbol_Table table;
        run(R"(
            def k = 42
            def f = fn v -> match v {
                _ is Null               => 'null',
                0                       => 'zero',
                (k)                     => 'k',
                n is Int    if: n < 100 => 'small-int',
                n is Int                => 'big-int',
                'hi'                    => 'hi-literal',
                s is String             => 'other-str',
                []                      => 'empty-array',
                [_]                     => 'singleton',
                [_, _]                  => 'pair',
                [_, _, ..._]            => 'long-array',
                _ is Map                => 'map',
                _                       => 'other'
            }
            def results = map [
                null, 0, 42, 7, 500, 'hi', 'bye',
                [], [1], [1, 2], [1, 2, 3, 4],
                {}, {key: 'v'}, true
            ] with f
        )",
            table);
        auto& r = lookup(table, "results")->raw_get<frst::Array>();
        std::vector<std::string> expected = {
            "null",       "zero",      "k",           "small-int", "big-int",
            "hi-literal", "other-str", "empty-array", "singleton", "pair",
            "long-array", "map",       "map",         "other"};
        REQUIRE(r.size() == expected.size());
        for (std::size_t i = 0; i < expected.size(); ++i)
            CHECK(r[i]->raw_get<frst::String>() == expected[i]);
    }
}
