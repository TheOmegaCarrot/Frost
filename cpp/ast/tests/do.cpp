#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>
#include <catch2/trompeloeil.hpp>

#include <frost/ast.hpp>
#include <frost/mock/mock-expression.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <fmt/format.h>

using namespace frst;
using namespace frst::ast;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;
using trompeloeil::_;

namespace
{

Expression::Ptr lit(Int v)
{
    return std::make_unique<Literal>(Value::create(v));
}

Expression::Ptr name_lookup(std::string_view n)
{
    return std::make_unique<Name_Lookup>(std::string{n});
}

std::string action_to_string(const Statement::Symbol_Action& action)
{
    return action.visit(Overload{
        [](const Statement::Definition& a) {
            return fmt::format("def:{}", a.name);
        },
        [](const Statement::Usage& a) {
            return fmt::format("use:{}", a.name);
        },
    });
}

std::vector<std::string> collect_sequence(const Statement& node)
{
    return node.symbol_sequence()
           | std::views::transform(&action_to_string)
           | std::ranges::to<std::vector>();
}

} // namespace

// =============================================================================
// Construction
// =============================================================================

TEST_CASE("Do_Block - construction")
{
    SECTION("Empty body throws")
    {
        CHECK_THROWS_AS(Do_Block{{}}, Frost_Unrecoverable_Error);
    }

    SECTION("Non-expression final statement throws")
    {
        // Define is a Statement but not an Expression — rejected as the tail
        std::vector<Statement::Ptr> body;
        body.push_back(std::make_unique<Define>("x", lit(1)));
        CHECK_THROWS_AS(Do_Block{std::move(body)}, Frost_Unrecoverable_Error);
    }

    SECTION("Single expression constructs successfully")
    {
        std::vector<Statement::Ptr> body;
        body.push_back(lit(42));
        CHECK_NOTHROW(Do_Block{std::move(body)});
    }

    SECTION("Expression with prefix statements constructs successfully")
    {
        std::vector<Statement::Ptr> body;
        body.push_back(std::make_unique<Define>("x", lit(1)));
        body.push_back(name_lookup("x"));
        CHECK_NOTHROW(Do_Block{std::move(body)});
    }
}

// =============================================================================
// Evaluate
// =============================================================================

TEST_CASE("Do_Block - evaluate")
{
    SECTION("Single expression body returns its value")
    {
        std::vector<Statement::Ptr> body;
        body.push_back(lit(42));
        Do_Block node{std::move(body)};

        Symbol_Table syms;
        auto result = node.evaluate(syms);
        REQUIRE(result->is<Int>());
        CHECK(result->get<Int>() == 42);
    }

    SECTION("Has read access to outer scope")
    {
        // If exec_table has no parent, Name_Lookup("x") would throw.
        Symbol_Table syms;
        syms.define("x", Value::create(Int{99}));

        std::vector<Statement::Ptr> body;
        body.push_back(name_lookup("x"));
        Do_Block node{std::move(body)};

        auto result = node.evaluate(syms);
        REQUIRE(result->is<Int>());
        CHECK(result->get<Int>() == 99);
    }

    SECTION("Local definition is visible in the value expression")
    {
        // If value_expr_ is evaluated against the outer syms instead of
        // exec_table, Name_Lookup("x") would fail — x is not in syms.
        std::vector<Statement::Ptr> body;
        body.push_back(std::make_unique<Define>("x", lit(7)));
        body.push_back(name_lookup("x"));
        Do_Block node{std::move(body)};

        Symbol_Table syms;
        auto result = node.evaluate(syms);
        REQUIRE(result->is<Int>());
        CHECK(result->get<Int>() == 7);
    }

    SECTION("Local definitions do not leak to outer scope")
    {
        // If prefix statements are executed against syms directly instead of
        // exec_table, "x" would appear in syms after the call.
        std::vector<Statement::Ptr> body;
        body.push_back(std::make_unique<Define>("x", lit(1)));
        body.push_back(name_lookup("x"));
        Do_Block node{std::move(body)};

        Symbol_Table syms;
        (void)node.evaluate(syms);
        CHECK_FALSE(syms.has("x"));
    }

    SECTION("Prefix statements execute in order")
    {
        // def y = x requires x to already be defined.
        // If statements ran in reverse, y = x would fail (x not yet defined).
        std::vector<Statement::Ptr> body;
        body.push_back(std::make_unique<Define>("x", lit(5)));
        body.push_back(std::make_unique<Define>("y", name_lookup("x")));
        body.push_back(name_lookup("y"));
        Do_Block node{std::move(body)};

        Symbol_Table syms;
        auto result = node.evaluate(syms);
        REQUIRE(result->is<Int>());
        CHECK(result->get<Int>() == 5);
    }

    SECTION("Statement error propagates; value expression is not evaluated")
    {
        auto bad = mock::Mock_Expression::make();
        auto never = mock::Mock_Expression::make();
        auto* bad_ptr = bad.get();
        auto* never_ptr = never.get();

        REQUIRE_CALL(*bad_ptr, evaluate(_))
            .THROW(Frost_Recoverable_Error{"prefix boom"});
        FORBID_CALL(*never_ptr, evaluate(_));

        std::vector<Statement::Ptr> body;
        body.push_back(std::make_unique<Define>("x", std::move(bad)));
        body.push_back(std::move(never)); // becomes value_expr_

        Do_Block node{std::move(body)};

        Symbol_Table syms;
        CHECK_THROWS_WITH(node.evaluate(syms),
                          ContainsSubstring("prefix boom"));
    }

    SECTION("Value expression error propagates")
    {
        auto bad = mock::Mock_Expression::make();
        auto* bad_ptr = bad.get();

        REQUIRE_CALL(*bad_ptr, evaluate(_))
            .THROW(Frost_Recoverable_Error{"value boom"});

        std::vector<Statement::Ptr> body;
        body.push_back(std::move(bad));
        Do_Block node{std::move(body)};

        Symbol_Table syms;
        CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("value boom"));
    }

    SECTION("Standalone expression as prefix statement is evaluated and discarded")
    {
        // do { a; b } — name("a") is a prefix expression-statement, not a Define.
        // Its result must be evaluated (touching exec_table) and discarded;
        // name("b") is the return value.
        Symbol_Table syms;
        syms.define("a", Value::create(Int{10}));
        syms.define("b", Value::create(Int{20}));

        std::vector<Statement::Ptr> body;
        body.push_back(name_lookup("a")); // prefix: evaluated, result discarded
        body.push_back(name_lookup("b")); // value_expr
        Do_Block node{std::move(body)};

        auto result = node.evaluate(syms);
        REQUIRE(result->is<Int>());
        CHECK(result->get<Int>() == 20);
    }

    SECTION("Can be evaluated multiple times")
    {
        std::vector<Statement::Ptr> body;
        body.push_back(lit(3));
        Do_Block node{std::move(body)};

        Symbol_Table syms;
        CHECK(node.evaluate(syms)->get<Int>() == 3);
        CHECK(node.evaluate(syms)->get<Int>() == 3);
    }
}

// =============================================================================
// Symbol sequence
// =============================================================================

TEST_CASE("Do_Block - symbol_sequence")
{
    SECTION("Single literal yields nothing")
    {
        std::vector<Statement::Ptr> body;
        body.push_back(lit(1));
        Do_Block node{std::move(body)};
        CHECK(collect_sequence(node).empty());
    }

    SECTION("Single name lookup yields its usage")
    {
        std::vector<Statement::Ptr> body;
        body.push_back(name_lookup("x"));
        Do_Block node{std::move(body)};
        CHECK(collect_sequence(node) == std::vector<std::string>{"use:x"});
    }

    SECTION("Local definition is not yielded")
    {
        // do { def x = 1; x } — x is defined and used locally; nothing escapes
        std::vector<Statement::Ptr> body;
        body.push_back(std::make_unique<Define>("x", lit(1)));
        body.push_back(name_lookup("x"));
        Do_Block node{std::move(body)};
        CHECK(collect_sequence(node).empty());
    }

    SECTION("Outer name usages are yielded")
    {
        // do { a; b } — prefix: [name(a)], value_expr: name(b)
        std::vector<Statement::Ptr> body;
        body.push_back(name_lookup("a"));
        body.push_back(name_lookup("b"));
        Do_Block node{std::move(body)};
        CHECK(collect_sequence(node)
              == std::vector<std::string>{"use:a", "use:b"});
    }

    SECTION("Local definition suppresses usage in value expression")
    {
        // do { def x = outer; x } — yields use:outer, not use:x
        std::vector<Statement::Ptr> body;
        body.push_back(std::make_unique<Define>("x", name_lookup("outer")));
        body.push_back(name_lookup("x"));
        Do_Block node{std::move(body)};
        CHECK(collect_sequence(node) == std::vector<std::string>{"use:outer"});
    }

    SECTION("Local definition suppresses usage in later prefix statement")
    {
        // do { def x = 1; x; x } — prefix: [Define(x,1), name(x)], value_expr:
        // name(x) All three usages of x are suppressed after the definition.
        std::vector<Statement::Ptr> body;
        body.push_back(std::make_unique<Define>("x", lit(1)));
        body.push_back(name_lookup("x")); // prefix statement
        body.push_back(name_lookup("x")); // value_expr
        Do_Block node{std::move(body)};
        CHECK(collect_sequence(node).empty());
    }

    SECTION("RHS of definition is yielded before the definition takes effect")
    {
        // do { def x = x_outer; x } — x_outer is from outside; x is local.
        // The RHS use:x_outer is seen before def:x, so x_outer is yielded.
        // The value_expr use:x is suppressed.
        std::vector<Statement::Ptr> body;
        body.push_back(std::make_unique<Define>("x", name_lookup("x_outer")));
        body.push_back(name_lookup("x"));
        Do_Block node{std::move(body)};
        CHECK(collect_sequence(node)
              == std::vector<std::string>{"use:x_outer"});
    }

    SECTION("Prefix and value usages appear in order")
    {
        // do { a; b; c } — prefix: [name(a), name(b)], value_expr: name(c)
        std::vector<Statement::Ptr> body;
        body.push_back(name_lookup("a"));
        body.push_back(name_lookup("b"));
        body.push_back(name_lookup("c"));
        Do_Block node{std::move(body)};
        CHECK(collect_sequence(node)
              == std::vector<std::string>{"use:a", "use:b", "use:c"});
    }

    SECTION("Multiple local definitions all accumulate in defns")
    {
        // do { def x = 1; def y = x; y }
        // Both x and y are local. The usage of x in def y's RHS must be
        // suppressed because x was already added to defns.
        // If defns failed to grow across successive defines, use:x would
        // escape incorrectly.
        std::vector<Statement::Ptr> body;
        body.push_back(std::make_unique<Define>("x", lit(1)));
        body.push_back(std::make_unique<Define>("y", name_lookup("x")));
        body.push_back(name_lookup("y"));
        Do_Block node{std::move(body)};
        CHECK(collect_sequence(node).empty());
    }

    SECTION("Inner block shadows outer local: nothing escapes")
    {
        // do { def x = 5; do { x; def x = 2; x } }
        // Inner's first use:x precedes inner's def:x → inner yields use:x.
        // Outer's def:x is already in defns when inner's use:x is processed
        // → suppressed. Inner's def:x and final use:x are purely local.
        // Net result: nothing escapes the outer block.
        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(name_lookup("x"));
        inner_body.push_back(std::make_unique<Define>("x", lit(2)));
        inner_body.push_back(name_lookup("x"));
        auto inner = std::make_unique<Do_Block>(std::move(inner_body));

        std::vector<Statement::Ptr> body;
        body.push_back(std::make_unique<Define>("x", lit(5)));
        body.push_back(std::move(inner));
        Do_Block node{std::move(body)};
        CHECK(collect_sequence(node).empty());
    }
}

// =============================================================================
// Nested do blocks
// =============================================================================

TEST_CASE("Do_Block - nested evaluate")
{
    SECTION("Usage before local definition reads from outer scope")
    {
        // do { x; def x = 5; x }
        // The first `x` reads the outer value (discarded); then local x = 5
        // is defined; the value expression returns the local 5.
        Symbol_Table syms;
        syms.define("x", Value::create(Int{99}));

        std::vector<Statement::Ptr> body;
        body.push_back(name_lookup("x"));                        // reads outer 99, discarded
        body.push_back(std::make_unique<Define>("x", lit(5)));   // defines local x
        body.push_back(name_lookup("x"));                        // returns local 5
        Do_Block node{std::move(body)};

        auto result = node.evaluate(syms);
        REQUIRE(result->is<Int>());
        CHECK(result->get<Int>() == 5);
        // Outer scope must not have been modified by the local define.
        CHECK(syms.lookup("x")->get<Int>() == 99);
    }

    SECTION("Inner block reads outer block's local")
    {
        // do { def x = 5; do { x } }
        // Inner block's exec_table must be a child of the outer exec_table,
        // not a child of the top-level syms directly.
        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(name_lookup("x"));
        auto inner = std::make_unique<Do_Block>(std::move(inner_body));

        std::vector<Statement::Ptr> body;
        body.push_back(std::make_unique<Define>("x", lit(5)));
        body.push_back(std::move(inner));
        Do_Block node{std::move(body)};

        Symbol_Table syms;
        auto result = node.evaluate(syms);
        REQUIRE(result->is<Int>());
        CHECK(result->get<Int>() == 5);
    }

    SECTION("Do block used as an expression on the RHS of a define")
    {
        // do { def x = do { 3 }; x }
        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(lit(3));
        auto inner = std::make_unique<Do_Block>(std::move(inner_body));

        std::vector<Statement::Ptr> body;
        body.push_back(std::make_unique<Define>("x", std::move(inner)));
        body.push_back(name_lookup("x"));
        Do_Block node{std::move(body)};

        Symbol_Table syms;
        auto result = node.evaluate(syms);
        REQUIRE(result->is<Int>());
        CHECK(result->get<Int>() == 3);
    }

    SECTION("Inner block locals do not leak to outer block scope")
    {
        // do { do { def inner = 1; inner } }
        // After evaluation, "inner" must not be accessible in syms.
        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(std::make_unique<Define>("inner", lit(1)));
        inner_body.push_back(name_lookup("inner"));
        auto inner = std::make_unique<Do_Block>(std::move(inner_body));

        std::vector<Statement::Ptr> body;
        body.push_back(std::move(inner));
        Do_Block node{std::move(body)};

        Symbol_Table syms;
        (void)node.evaluate(syms);
        CHECK_FALSE(syms.has("inner"));
    }
}

TEST_CASE("Do_Block - nested symbol_sequence")
{
    SECTION("Usage before local definition escapes as an outer reference")
    {
        // do { x; def x = 5; x }
        // The first `x` precedes `def x`, so it is a usage of an outer `x`.
        // The final `x` is suppressed by the local definition.
        std::vector<Statement::Ptr> body;
        body.push_back(name_lookup("x"));
        body.push_back(std::make_unique<Define>("x", lit(5)));
        body.push_back(name_lookup("x"));
        Do_Block node{std::move(body)};
        CHECK(collect_sequence(node) == std::vector<std::string>{"use:x"});
    }

    SECTION("Outer local definition suppresses inner block's usage of same name")
    {
        // do { def x = 5; do { x; y } }
        // Inner's use:x is absorbed by the outer def:x.
        // Inner's use:y has no definition anywhere → escapes.
        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(name_lookup("x"));
        inner_body.push_back(name_lookup("y"));
        auto inner = std::make_unique<Do_Block>(std::move(inner_body));

        std::vector<Statement::Ptr> body;
        body.push_back(std::make_unique<Define>("x", lit(5)));
        body.push_back(std::move(inner));
        Do_Block node{std::move(body)};
        CHECK(collect_sequence(node) == std::vector<std::string>{"use:y"});
    }

    SECTION("Inner block usages propagate through outer block")
    {
        // do { do { a }; b }
        // Inner's use:a must be visible to an enclosing Lambda's capture analysis.
        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(name_lookup("a"));
        auto inner = std::make_unique<Do_Block>(std::move(inner_body));

        std::vector<Statement::Ptr> body;
        body.push_back(std::move(inner)); // prefix
        body.push_back(name_lookup("b")); // value_expr
        Do_Block node{std::move(body)};
        CHECK(collect_sequence(node)
              == std::vector<std::string>{"use:a", "use:b"});
    }
}

// =============================================================================
// node_label / walk / children
// =============================================================================

TEST_CASE("Do_Block - node_label and walk")
{
    SECTION("node_label returns Do_Block")
    {
        std::vector<Statement::Ptr> body;
        body.push_back(lit(1));
        Do_Block node{std::move(body)};
        CHECK(node.node_label() == "Do_Block");
    }

    SECTION("walk visits prefix nodes then value expression, in order")
    {
        auto a = std::make_unique<Name_Lookup>("a");
        auto b = std::make_unique<Name_Lookup>("b");
        auto c = std::make_unique<Name_Lookup>("c");
        const auto* a_ptr = a.get();
        const auto* b_ptr = b.get();
        const auto* c_ptr = c.get();

        std::vector<Statement::Ptr> body;
        body.push_back(std::move(a)); // prefix
        body.push_back(std::move(b)); // prefix
        body.push_back(std::move(c)); // value_expr

        Do_Block node{std::move(body)};

        std::vector<const Statement*> walked;
        for (const auto* n : node.walk())
            walked.push_back(n);

        REQUIRE(walked.size() == 4);
        CHECK(walked[0] == &node);
        CHECK(walked[1] == a_ptr);
        CHECK(walked[2] == b_ptr);
        CHECK(walked[3] == c_ptr);
    }

    SECTION("Single-expression body: walk visits block then expression")
    {
        auto expr = std::make_unique<Name_Lookup>("x");
        const auto* expr_ptr = expr.get();

        std::vector<Statement::Ptr> body;
        body.push_back(std::move(expr));
        Do_Block node{std::move(body)};

        std::vector<const Statement*> walked;
        for (const auto* n : node.walk())
            walked.push_back(n);

        REQUIRE(walked.size() == 2);
        CHECK(walked[0] == &node);
        CHECK(walked[1] == expr_ptr);
    }

    SECTION("walk descends into grandchildren")
    {
        // do { do { x } }
        // outer: prefix=[], value_expr=inner Do_Block
        // inner: prefix=[], value_expr=Name_Lookup("x")
        // walk should yield: outer, inner, Name_Lookup("x")
        auto leaf = std::make_unique<Name_Lookup>("x");
        const auto* leaf_ptr = leaf.get();

        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(std::move(leaf));
        auto inner = std::make_unique<Do_Block>(std::move(inner_body));
        const auto* inner_ptr = inner.get();

        std::vector<Statement::Ptr> body;
        body.push_back(std::move(inner));
        Do_Block node{std::move(body)};

        std::vector<const Statement*> walked;
        for (const auto* n : node.walk())
            walked.push_back(n);

        REQUIRE(walked.size() == 3);
        CHECK(walked[0] == &node);
        CHECK(walked[1] == inner_ptr);
        CHECK(walked[2] == leaf_ptr);
    }
}
