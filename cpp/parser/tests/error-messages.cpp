#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/parser.hpp>

using namespace Catch::Matchers;

namespace
{
// Asserts that the input fails to parse and returns the error text.
std::string parse_error(const std::string& input)
{
    auto result = frst::parse_program(input, "<test>");
    REQUIRE_FALSE(result.has_value());
    return result.error();
}
} // namespace

TEST_CASE("Parser error messages - boolean operators")
{
    SECTION("&& suggests 'and'")
    {
        auto err = parse_error("a && b");
        CHECK_THAT(err, ContainsSubstring("use 'and' instead of '&&'"));
    }

    SECTION("|| suggests 'or'")
    {
        auto err = parse_error("a || b");
        CHECK_THAT(err, ContainsSubstring("use 'or' instead of '||'"));
    }

    SECTION("&& hint fires inside an expression context too")
    {
        // The incomplete expression `a` becomes a statement, then `&&` is
        // seen at the program level — same hint regardless.
        auto err = parse_error("def x = a\n&& b");
        CHECK_THAT(err, ContainsSubstring("use 'and' instead of '&&'"));
    }
}

TEST_CASE("Parser error messages - assignment without def")
{
    SECTION("x = 1 suggests def")
    {
        auto err = parse_error("x = 1");
        CHECK_THAT(err, ContainsSubstring("use 'def name = value'"));
    }

    SECTION("hint fires for any bare = at program level")
    {
        auto err = parse_error("foo = 42");
        CHECK_THAT(err, ContainsSubstring("use 'def name = value'"));
    }
}

TEST_CASE("Parser error messages - else if instead of elif")
{
    SECTION("else if suggests elif")
    {
        auto err = parse_error("if a: 1 else if b: 2 else: 3");
        CHECK_THAT(err, ContainsSubstring("use 'elif' instead of 'else if'"));
    }

    SECTION("valid elif still parses")
    {
        auto result = frst::parse_program("if true: 1 elif false: 2 else: 3", "<test>");
        CHECK(result.has_value());
    }

    SECTION("valid else still parses")
    {
        auto result = frst::parse_program("if true: 1 else: 2", "<test>");
        CHECK(result.has_value());
    }
}

TEST_CASE("Parser error messages - string literal rule name")
{
    SECTION("unclosed double-quoted string says 'string literal'")
    {
        auto err = parse_error("\"hello");
        CHECK_THAT(err, ContainsSubstring("string literal"));
        CHECK_THAT(err, !ContainsSubstring("raw_double"));
    }

    SECTION("unclosed single-quoted string says 'string literal'")
    {
        auto err = parse_error("'hello");
        CHECK_THAT(err, ContainsSubstring("string literal"));
        CHECK_THAT(err, !ContainsSubstring("raw_single"));
    }
}

TEST_CASE("Parser error messages - map entry value label")
{
    SECTION("missing value after colon says 'map entry value'")
    {
        auto err = parse_error("{a: }");
        CHECK_THAT(err, ContainsSubstring("map entry value"));
    }
}

TEST_CASE("Parser error messages - closing delimiter labels")
{
    SECTION("unclosed call says 'closing ')'")
    {
        // Empty arg list: expected_call_arguments fires immediately
        auto err = parse_error("f(");
        CHECK_THAT(err, ContainsSubstring("closing ')'"));
    }

    SECTION("unclosed index says 'closing ']'")
    {
        auto err = parse_error("a[");
        CHECK_THAT(err, ContainsSubstring("closing ']'"));
    }

    SECTION("call with args: missing ')' is still diagnosed")
    {
        auto err = parse_error("f(a");
        CHECK_THAT(err, ContainsSubstring("expected ')'"));
    }
}

TEST_CASE("Parser error messages - foreign language keywords")
{
    SECTION("for loop")
    {
        auto err = parse_error("for x in items { print(x) }");
        CHECK_THAT(err, ContainsSubstring("unexpected 'for'"));
    }

    SECTION("while loop")
    {
        auto err = parse_error("while true: print(1)");
        CHECK_THAT(err, ContainsSubstring("unexpected 'while'"));
    }

    SECTION("let binding")
    {
        auto err = parse_error("let x = 5");
        CHECK_THAT(err, ContainsSubstring("'def' instead of 'let'"));
    }

    SECTION("var binding")
    {
        auto err = parse_error("var x = 5");
        CHECK_THAT(err, ContainsSubstring("'def' instead of 'var'"));
    }

    SECTION("is in if condition suggests match")
    {
        auto err = parse_error("if x is Array: 1");
        CHECK_THAT(err, ContainsSubstring("match patterns"));
        CHECK_THAT(err, ContainsSubstring("is_array()"));
    }

    SECTION("is in elif condition suggests match")
    {
        auto err = parse_error("if false: 0 elif x is Int: 1");
        CHECK_THAT(err, ContainsSubstring("match patterns"));
    }

    SECTION("iterative keywords in @ pipelines suggest functional equivalents")
    {
        auto err_map = parse_error("[1] @ map(fn n -> n)");
        CHECK_THAT(err_map, ContainsSubstring("'transform'"));

        auto err_filter = parse_error("[1] @ filter(fn n -> true)");
        CHECK_THAT(err_filter, ContainsSubstring("'select'"));

        auto err_reduce = parse_error("[1] @ reduce(fn a, b -> a)");
        CHECK_THAT(err_reduce, ContainsSubstring("'fold'"));

        auto err_foreach = parse_error("[1] @ foreach(print)");
        CHECK_THAT(err_foreach, ContainsSubstring("'foreach' cannot"));
    }

    SECTION("identifiers starting with foreign keywords still work")
    {
        // `format`, `formula`, `letter`, `variable` must NOT trigger errors.
        auto ok = [](const std::string& input) {
            return frst::parse_program(input, "<test>").has_value();
        };
        CHECK(ok("def format = 1"));
        CHECK(ok("def formula = 2"));
        CHECK(ok("def letter = 3"));
        CHECK(ok("def variable = 4"));
        CHECK(ok("def whileloop = 5"));
        CHECK(ok("def foreach_item = 6"));
    }
}

TEST_CASE("Parser error messages - return keyword")
{
    SECTION("return suggests implicit return")
    {
        auto err = parse_error("return 42");
        CHECK_THAT(err, ContainsSubstring("unexpected 'return'"));
    }

    SECTION("return inside a lambda body")
    {
        auto err = parse_error("def f = fn x -> { return x }");
        CHECK_THAT(err, ContainsSubstring("unexpected 'return'"));
    }
}

TEST_CASE("Parser error messages - unicode in comments")
{
    SECTION("em dash in comment")
    {
        auto result = frst::parse_program(
            "# this is a comment \xe2\x80\x94 with an em dash\nprint(1)", "<test>");
        CHECK(result.has_value());
    }

    SECTION("curly quotes in comment")
    {
        auto result =
            frst::parse_program("# \xe2\x80\x9chello\xe2\x80\x9d\nprint(1)", "<test>");
        CHECK(result.has_value());
    }
}

TEST_CASE("Parser error messages - reassignment inside blocks")
{
    SECTION("reassignment in lambda body")
    {
        auto err = parse_error("def f = fn x -> { x = 10; x }");
        CHECK_THAT(err, ContainsSubstring("'def name = value'"));
    }

    SECTION("reassignment in do block")
    {
        auto err = parse_error("do { x = 10; x }");
        CHECK_THAT(err, ContainsSubstring("'def name = value'"));
    }

    SECTION("reassignment at top level still works")
    {
        auto err = parse_error("x = 10");
        CHECK_THAT(err, ContainsSubstring("'def name = value'"));
    }

    SECTION("valid assignment-like code still parses")
    {
        // `def x = 10` inside blocks is fine
        auto ok = [](const std::string& input) {
            return frst::parse_program(input, "<test>").has_value();
        };
        CHECK(ok("def f = fn x -> { def y = x + 1; y }"));
        CHECK(ok("do { def x = 10; x }"));
    }
}

TEST_CASE("Parser error messages - do block suggestion")
{
    SECTION("def in if branch suggests do")
    {
        auto err = parse_error("if true: { def x = 1; x }");
        CHECK_THAT(err, ContainsSubstring("use 'do { ... }' for a block"));
    }

    SECTION("def in else branch suggests do")
    {
        auto err = parse_error("if false: 0 else: { def x = 1; x }");
        CHECK_THAT(err, ContainsSubstring("use 'do { ... }' for a block"));
    }

    SECTION("def in match arm suggests do")
    {
        auto err = parse_error("match 1 { n => { def x = n; x } }");
        CHECK_THAT(err, ContainsSubstring("use 'do { ... }' for a block"));
    }

    SECTION("defn in map position suggests do")
    {
        auto err = parse_error("{ defn foo() -> 1 }");
        CHECK_THAT(err, ContainsSubstring("use 'do { ... }' for a block"));
    }

    SECTION("export in map position suggests do")
    {
        auto err = parse_error("{ export def x = 1 }");
        CHECK_THAT(err, ContainsSubstring("use 'do { ... }' for a block"));
    }

    SECTION("does not fire for valid map literals")
    {
        auto ok = [](const std::string& input) {
            return frst::parse_program(input, "<test>").has_value();
        };
        CHECK(ok("def m = {}"));
        CHECK(ok("def m = { foo: 42 }"));
        CHECK(ok("def m = { [1]: 'one' }"));
        CHECK(ok("def m = { default: 1, definition: 2 }"));
        CHECK(ok("def m = { exported: true }"));
        CHECK(ok(R"(def m = { ["def"]: 42 })"));
    }

    SECTION("does not fire in lambda block bodies")
    {
        auto ok = [](const std::string& input) {
            return frst::parse_program(input, "<test>").has_value();
        };
        CHECK(ok("def f = fn -> { def x = 1; x }"));
        CHECK(ok("def f = fn x -> { def y = x + 1; y }"));
    }

    SECTION("does not fire in do blocks")
    {
        auto ok = [](const std::string& input) {
            return frst::parse_program(input, "<test>").has_value();
        };
        CHECK(ok("do { def x = 1; x }"));
        CHECK(ok("if true: do { def x = 1; x }"));
    }
}
