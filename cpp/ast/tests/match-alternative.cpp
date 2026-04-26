#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>
#include <catch2/trompeloeil.hpp>

#include <frost/mock/mock-match-pattern.hpp>
#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/ast/match-alternative.hpp>
#include <frost/ast/match-array.hpp>
#include <frost/ast/match-binding.hpp>
#include <frost/ast/match-value.hpp>
#include <frost/ast/name-lookup.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <memory>
#include <ranges>
#include <vector>

using namespace frst;
using namespace frst::ast;
using namespace std::literals;

using trompeloeil::_;

namespace
{

// Helper: build a Match_Alternative from a vector of sub-patterns.
Match_Alternative::Ptr make_alt(std::vector<Match_Pattern::Ptr> alts)
{
    return std::make_unique<Match_Alternative>(AST_Node::no_range,
                                               std::move(alts));
}

// Helper: build a binding pattern (named or discard).
Match_Pattern::Ptr binding(std::optional<std::string> name)
{
    return std::make_unique<Match_Binding>(AST_Node::no_range, std::move(name),
                                           std::nullopt);
}

} // namespace

// =============================================================================
// Constructor validation
// =============================================================================

TEST_CASE("Match_Alternative: requires at least 2 alternatives")
{
    std::vector<Match_Pattern::Ptr> one;
    one.push_back(binding("x"s));
    CHECK_THROWS(make_alt(std::move(one)));
}

TEST_CASE("Match_Alternative: accepts 2 alternatives with matching bindings")
{
    std::vector<Match_Pattern::Ptr> alts;
    alts.push_back(binding("x"s));
    alts.push_back(binding("x"s));
    CHECK_NOTHROW(make_alt(std::move(alts)));
}

TEST_CASE("Match_Alternative: accepts alternatives with no bindings")
{
    // Two discard patterns -- both bind nothing, so they agree.
    std::vector<Match_Pattern::Ptr> alts;
    alts.push_back(binding(std::nullopt));
    alts.push_back(binding(std::nullopt));
    CHECK_NOTHROW(make_alt(std::move(alts)));
}

TEST_CASE("Match_Alternative: rejects mismatched bindings")
{
    std::vector<Match_Pattern::Ptr> alts;
    alts.push_back(binding("x"s));
    alts.push_back(binding("y"s));
    CHECK_THROWS_AS(make_alt(std::move(alts)), Frost_Unrecoverable_Error);
}

TEST_CASE(
    "Match_Alternative: rejects when one alternative binds and another doesn't")
{
    std::vector<Match_Pattern::Ptr> alts;
    alts.push_back(binding("x"s));
    alts.push_back(binding(std::nullopt));
    CHECK_THROWS_AS(make_alt(std::move(alts)), Frost_Unrecoverable_Error);
}

TEST_CASE("Match_Alternative: rejects mismatch in 3rd alternative")
{
    std::vector<Match_Pattern::Ptr> alts;
    alts.push_back(binding("x"s));
    alts.push_back(binding("x"s));
    alts.push_back(binding("y"s));
    CHECK_THROWS_AS(make_alt(std::move(alts)), Frost_Unrecoverable_Error);
}

TEST_CASE("Match_Alternative: accepts 3 alternatives with matching bindings")
{
    std::vector<Match_Pattern::Ptr> alts;
    alts.push_back(binding("x"s));
    alts.push_back(binding("x"s));
    alts.push_back(binding("x"s));
    CHECK_NOTHROW(make_alt(std::move(alts)));
}

// =============================================================================
// do_try_match
// =============================================================================

TEST_CASE("Match_Alternative: first matching alternative wins")
{
    Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};

    auto a = mock::Mock_Match_Pattern::make();
    auto b = mock::Mock_Match_Pattern::make();

    trompeloeil::sequence seq;
    REQUIRE_CALL(*a, do_try_match(_, _)).IN_SEQUENCE(seq).RETURN(true);
    FORBID_CALL(*b, do_try_match(_, _));

    std::vector<Match_Pattern::Ptr> alts;
    alts.push_back(std::move(a));
    alts.push_back(std::move(b));
    auto pat = make_alt(std::move(alts));

    CHECK(pat->try_match(ctx, Value::create(1_f)));
}

TEST_CASE("Match_Alternative: falls through to second alternative")
{
    Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};

    auto a = mock::Mock_Match_Pattern::make();
    auto b = mock::Mock_Match_Pattern::make();

    trompeloeil::sequence seq;
    REQUIRE_CALL(*a, do_try_match(_, _)).IN_SEQUENCE(seq).RETURN(false);
    REQUIRE_CALL(*b, do_try_match(_, _)).IN_SEQUENCE(seq).RETURN(true);

    std::vector<Match_Pattern::Ptr> alts;
    alts.push_back(std::move(a));
    alts.push_back(std::move(b));
    auto pat = make_alt(std::move(alts));

    CHECK(pat->try_match(ctx, Value::create(1_f)));
}

TEST_CASE("Match_Alternative: all alternatives fail returns false")
{
    Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};

    auto a = mock::Mock_Match_Pattern::make();
    auto b = mock::Mock_Match_Pattern::make();

    REQUIRE_CALL(*a, do_try_match(_, _)).RETURN(false);
    REQUIRE_CALL(*b, do_try_match(_, _)).RETURN(false);

    std::vector<Match_Pattern::Ptr> alts;
    alts.push_back(std::move(a));
    alts.push_back(std::move(b));
    auto pat = make_alt(std::move(alts));

    CHECK_FALSE(pat->try_match(ctx, Value::create(1_f)));
}

TEST_CASE("Match_Alternative: bindings from matched alternative are visible")
{
    Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};

    // First alt fails, second succeeds and binds "x"
    auto a = mock::Mock_Match_Pattern::make();
    auto b = mock::Mock_Match_Pattern::make();

    REQUIRE_CALL(*a, do_try_match(_, _)).RETURN(false);
    REQUIRE_CALL(*b, do_try_match(_, _))
        .SIDE_EFFECT(_1.symbols.define("x", Value::create(42_f)))
        .RETURN(true);

    std::vector<Match_Pattern::Ptr> alts;
    alts.push_back(std::move(a));
    alts.push_back(std::move(b));
    auto pat = make_alt(std::move(alts));

    CHECK(pat->try_match(ctx, Value::create(1_f)));
    REQUIRE(syms.has("x"));
    CHECK(syms.lookup("x")->raw_get<Int>() == 42_f);
}

TEST_CASE(
    "Match_Alternative: partial bindings from failed alternative don't leak")
{
    Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};

    // First alt binds "x" then fails; second alt succeeds with "x"
    auto a = mock::Mock_Match_Pattern::make();
    auto b = mock::Mock_Match_Pattern::make();

    REQUIRE_CALL(*a, do_try_match(_, _))
        .SIDE_EFFECT(_1.symbols.define("x", Value::create(99_f)))
        .RETURN(false);
    REQUIRE_CALL(*b, do_try_match(_, _))
        .SIDE_EFFECT(_1.symbols.define("x", Value::create(42_f)))
        .RETURN(true);

    std::vector<Match_Pattern::Ptr> alts;
    alts.push_back(std::move(a));
    alts.push_back(std::move(b));
    auto pat = make_alt(std::move(alts));

    CHECK(pat->try_match(ctx, Value::create(1_f)));
    REQUIRE(syms.has("x"));
    CHECK(syms.lookup("x")->raw_get<Int>() == 42_f);
}

TEST_CASE("Match_Alternative: no bindings leak when all alternatives fail")
{
    Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};

    auto a = mock::Mock_Match_Pattern::make();
    auto b = mock::Mock_Match_Pattern::make();

    REQUIRE_CALL(*a, do_try_match(_, _))
        .SIDE_EFFECT(_1.symbols.define("x", Value::create(1_f)))
        .RETURN(false);
    REQUIRE_CALL(*b, do_try_match(_, _))
        .SIDE_EFFECT(_1.symbols.define("x", Value::create(2_f)))
        .RETURN(false);

    std::vector<Match_Pattern::Ptr> alts;
    alts.push_back(std::move(a));
    alts.push_back(std::move(b));
    auto pat = make_alt(std::move(alts));

    CHECK_FALSE(pat->try_match(ctx, Value::create(1_f)));
    CHECK_FALSE(syms.has("x"));
}

TEST_CASE("Match_Alternative: errors from alternatives propagate")
{
    Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};

    auto a = mock::Mock_Match_Pattern::make();
    auto b = mock::Mock_Match_Pattern::make();

    REQUIRE_CALL(*a, do_try_match(_, _)).THROW(Frost_Recoverable_Error{"boom"});
    FORBID_CALL(*b, do_try_match(_, _));

    std::vector<Match_Pattern::Ptr> alts;
    alts.push_back(std::move(a));
    alts.push_back(std::move(b));
    auto pat = make_alt(std::move(alts));

    CHECK_THROWS_AS(pat->try_match(ctx, Value::create(1_f)),
                    Frost_Recoverable_Error);
}

TEST_CASE(
    "Match_Alternative: target value passed to each alternative unchanged")
{
    Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};

    auto target = Value::create(42_f);

    auto a = mock::Mock_Match_Pattern::make();
    auto b = mock::Mock_Match_Pattern::make();

    REQUIRE_CALL(*a, do_try_match(_, _))
        .WITH(_2.get() == target.get())
        .RETURN(false);
    REQUIRE_CALL(*b, do_try_match(_, _))
        .WITH(_2.get() == target.get())
        .RETURN(true);

    std::vector<Match_Pattern::Ptr> alts;
    alts.push_back(std::move(a));
    alts.push_back(std::move(b));
    auto pat = make_alt(std::move(alts));

    CHECK(pat->try_match(ctx, target));
}

// =============================================================================
// symbol_sequence
// =============================================================================

TEST_CASE("Match_Alternative: symbol_sequence yields definitions from all "
          "alternatives")
{
    // Both alternatives bind "x". The sequence should contain two
    // Definition{x} -- consumers handle dedup.
    std::vector<Match_Pattern::Ptr> alts;
    alts.push_back(binding("x"s));
    alts.push_back(binding("x"s));
    auto pat = make_alt(std::move(alts));

    auto actions = pat->symbol_sequence() | std::ranges::to<std::vector>();
    REQUIRE(actions.size() == 2);
    for (const auto& action : actions)
    {
        auto* def = std::get_if<AST_Node::Definition>(&action);
        REQUIRE(def);
        CHECK(def->name == "x");
    }
}

TEST_CASE("Match_Alternative: symbol_sequence yields usages from all "
          "alternatives")
{
    // Use Match_Value(Name_Lookup) to produce a Usage in each alternative.
    auto val1 = std::make_unique<Match_Value>(
        AST_Node::no_range,
        std::make_unique<Name_Lookup>(AST_Node::no_range, "outer_a"s));
    auto val2 = std::make_unique<Match_Value>(
        AST_Node::no_range,
        std::make_unique<Name_Lookup>(AST_Node::no_range, "outer_b"s));

    std::vector<Match_Pattern::Ptr> alts;
    alts.push_back(std::move(val1));
    alts.push_back(std::move(val2));
    auto pat = make_alt(std::move(alts));

    auto actions = pat->symbol_sequence() | std::ranges::to<std::vector>();
    REQUIRE(actions.size() == 2);

    auto* u1 = std::get_if<AST_Node::Usage>(&actions[0]);
    REQUIRE(u1);
    CHECK(u1->name == "outer_a");

    auto* u2 = std::get_if<AST_Node::Usage>(&actions[1]);
    REQUIRE(u2);
    CHECK(u2->name == "outer_b");
}

TEST_CASE("Match_Alternative: symbol_sequence with no-binding alternatives "
          "yields nothing")
{
    std::vector<Match_Pattern::Ptr> alts;
    alts.push_back(binding(std::nullopt));
    alts.push_back(binding(std::nullopt));
    auto pat = make_alt(std::move(alts));

    auto actions = pat->symbol_sequence() | std::ranges::to<std::vector>();
    CHECK(actions.empty());
}

// =============================================================================
// children
// =============================================================================

TEST_CASE("Match_Alternative: children exposes all alternatives")
{
    auto a = mock::Mock_Match_Pattern::make();
    auto b = mock::Mock_Match_Pattern::make();
    auto c = mock::Mock_Match_Pattern::make();
    auto* a_raw = a.get();
    auto* b_raw = b.get();
    auto* c_raw = c.get();

    std::vector<Match_Pattern::Ptr> alts;
    alts.push_back(std::move(a));
    alts.push_back(std::move(b));
    alts.push_back(std::move(c));
    auto pat = make_alt(std::move(alts));

    auto kids = pat->children() | std::ranges::to<std::vector>();
    REQUIRE(kids.size() == 3);
    CHECK(kids[0].node == a_raw);
    CHECK(kids[1].node == b_raw);
    CHECK(kids[2].node == c_raw);
}

// =============================================================================
// node_label
// =============================================================================

TEST_CASE("Match_Alternative: node_label")
{
    std::vector<Match_Pattern::Ptr> alts;
    alts.push_back(binding("x"s));
    alts.push_back(binding("x"s));
    auto pat = make_alt(std::move(alts));

    CHECK(pat->node_label() == "Match_Alternative");
}

TEST_CASE("Match_Alternative: multiple bindings per alternative are all merged")
{
    // Each alternative is an array pattern [x, y] that binds two names.
    // On a successful match, both must appear in the caller's symbol table.
    Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};

    // First alt: array pattern expecting [_, _] -- won't match a 3-element
    // array
    std::vector<Match_Pattern::Ptr> sub1;
    sub1.push_back(binding("x"s));
    sub1.push_back(binding("y"s));
    auto arr1 = std::make_unique<Match_Array>(AST_Node::no_range,
                                              std::move(sub1), std::nullopt);

    // Second alt: array pattern expecting [_, _, _] -- matches [1, 2, 3]
    std::vector<Match_Pattern::Ptr> sub2;
    sub2.push_back(binding("x"s));
    sub2.push_back(binding("y"s));
    sub2.push_back(binding(std::nullopt)); // discard third element
    auto arr2 = std::make_unique<Match_Array>(AST_Node::no_range,
                                              std::move(sub2), std::nullopt);

    std::vector<Match_Pattern::Ptr> alts;
    alts.push_back(std::move(arr1));
    alts.push_back(std::move(arr2));
    auto pat = make_alt(std::move(alts));

    auto target = Value::create(Array{
        Value::create(10_f),
        Value::create(20_f),
        Value::create(30_f),
    });

    CHECK(pat->try_match(ctx, target));
    REQUIRE(syms.has("x"));
    REQUIRE(syms.has("y"));
    CHECK(syms.lookup("x")->raw_get<Int>() == 10_f);
    CHECK(syms.lookup("y")->raw_get<Int>() == 20_f);
}

TEST_CASE(
    "Match_Alternative: error message names alternatives and binding sets")
{
    using Catch::Matchers::ContainsSubstring;

    // Alt 1 binds {a, b}, alt 2 binds {x} -- should mention both.
    std::vector<Match_Pattern::Ptr> sub1;
    sub1.push_back(binding("a"s));
    sub1.push_back(binding("b"s));
    auto arr1 = std::make_unique<Match_Array>(AST_Node::no_range,
                                              std::move(sub1), std::nullopt);

    std::vector<Match_Pattern::Ptr> alts;
    alts.push_back(std::move(arr1));
    alts.push_back(binding("x"s));

    CHECK_THROWS_WITH(make_alt(std::move(alts)),
                      ContainsSubstring("alternative 1")
                          && ContainsSubstring("a, b")
                          && ContainsSubstring("alternative 2")
                          && ContainsSubstring("x"));
}
