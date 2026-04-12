#include <catch2/catch_test_macros.hpp>
#include <catch2/trompeloeil.hpp>

#include <frost/mock/mock-match-pattern.hpp>
#include <frost/mock/mock-symbol-table.hpp>
#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/ast/match-array.hpp>
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

// Helper: build a Match_Array from a vector of sub-patterns and an optional
// rest clause.
Match_Array::Ptr make_array(std::vector<Match_Pattern::Ptr> subpatterns,
                            std::optional<Match_Array::Rest> rest = std::nullopt)
{
    return std::make_unique<Match_Array>(
        AST_Node::no_range, std::move(subpatterns), std::move(rest));
}

// Convenience: a Match_Array::Rest that discards the tail.
Match_Array::Rest discard_rest()
{
    return Match_Array::Rest{std::nullopt};
}

// Convenience: a Match_Array::Rest that binds the tail to the given name.
Match_Array::Rest named_rest(std::string name)
{
    return Match_Array::Rest{std::move(name)};
}

// Convenience: a Value_Ptr holding an Array of ints 1..n.
Value_Ptr int_array(std::size_t n)
{
    Array arr;
    for (std::size_t i = 0; i < n; ++i)
        arr.push_back(Value::create(static_cast<Int>(i + 1)));
    return Value::create(std::move(arr));
}

} // namespace

TEST_CASE("Match_Array: non-array match target fails without calling subpatterns")
{
    // The type check must run first. No sub-pattern should be consulted,
    // so FORBID_CALL the mock.
    mock::Mock_Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};

    auto inner = mock::Mock_Match_Pattern::make();
    FORBID_CALL(*inner, do_try_match(_, _));

    std::vector<Match_Pattern::Ptr> subs;
    subs.push_back(std::move(inner));
    auto pat = make_array(std::move(subs));

    SECTION("Int match target") { CHECK_FALSE(pat->try_match(ctx, Value::create(42_f))); }
    SECTION("String match target") { CHECK_FALSE(pat->try_match(ctx, Value::create("hi"s))); }
    SECTION("Map match target") { CHECK_FALSE(pat->try_match(ctx, Value::create(Map{}))); }
    SECTION("Null match target") { CHECK_FALSE(pat->try_match(ctx, Value::null())); }
    SECTION("Bool match target") { CHECK_FALSE(pat->try_match(ctx, Value::create(true))); }
}

TEST_CASE("Match_Array: exact-length matching without rest")
{
    mock::Mock_Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};

    SECTION("Empty pattern matches empty array")
    {
        auto pat = make_array({});
        CHECK(pat->try_match(ctx, Value::create(Array{})));
    }

    SECTION("Empty pattern does not match non-empty array")
    {
        auto pat = make_array({});
        CHECK_FALSE(pat->try_match(ctx, int_array(1)));
    }

    SECTION("Non-empty pattern does not match empty array")
    {
        auto inner = mock::Mock_Match_Pattern::make();
        FORBID_CALL(*inner, do_try_match(_, _));

        std::vector<Match_Pattern::Ptr> subs;
        subs.push_back(std::move(inner));
        auto pat = make_array(std::move(subs));
        CHECK_FALSE(pat->try_match(ctx, Value::create(Array{})));
    }

    SECTION("Length mismatch (pattern shorter) returns false")
    {
        auto a = mock::Mock_Match_Pattern::make();
        FORBID_CALL(*a, do_try_match(_, _));

        std::vector<Match_Pattern::Ptr> subs;
        subs.push_back(std::move(a));
        auto pat = make_array(std::move(subs));

        CHECK_FALSE(pat->try_match(ctx, int_array(3))); // [1, 2, 3]
    }

    SECTION("Length mismatch (pattern longer) returns false")
    {
        auto a = mock::Mock_Match_Pattern::make();
        auto b = mock::Mock_Match_Pattern::make();
        FORBID_CALL(*a, do_try_match(_, _));
        FORBID_CALL(*b, do_try_match(_, _));

        std::vector<Match_Pattern::Ptr> subs;
        subs.push_back(std::move(a));
        subs.push_back(std::move(b));
        auto pat = make_array(std::move(subs));

        CHECK_FALSE(pat->try_match(ctx, int_array(1))); // [1]
    }

    SECTION("Each subpattern receives the corresponding element in order")
    {
        auto a = mock::Mock_Match_Pattern::make();
        auto b = mock::Mock_Match_Pattern::make();
        auto c = mock::Mock_Match_Pattern::make();

        trompeloeil::sequence seq;
        REQUIRE_CALL(*a, do_try_match(_, _))
            .WITH(_2->template raw_get<Int>() == 1_f)
            .IN_SEQUENCE(seq)
            .RETURN(true);
        REQUIRE_CALL(*b, do_try_match(_, _))
            .WITH(_2->template raw_get<Int>() == 2_f)
            .IN_SEQUENCE(seq)
            .RETURN(true);
        REQUIRE_CALL(*c, do_try_match(_, _))
            .WITH(_2->template raw_get<Int>() == 3_f)
            .IN_SEQUENCE(seq)
            .RETURN(true);

        std::vector<Match_Pattern::Ptr> subs;
        subs.push_back(std::move(a));
        subs.push_back(std::move(b));
        subs.push_back(std::move(c));
        auto pat = make_array(std::move(subs));

        CHECK(pat->try_match(ctx, int_array(3))); // [1, 2, 3]
    }
}

TEST_CASE("Match_Array: subpattern failure stops the match")
{
    mock::Mock_Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};

    SECTION("First subpattern fails: later ones are not tried")
    {
        auto a = mock::Mock_Match_Pattern::make();
        auto b = mock::Mock_Match_Pattern::make();

        REQUIRE_CALL(*a, do_try_match(_, _)).RETURN(false);
        FORBID_CALL(*b, do_try_match(_, _));

        std::vector<Match_Pattern::Ptr> subs;
        subs.push_back(std::move(a));
        subs.push_back(std::move(b));
        auto pat = make_array(std::move(subs));

        CHECK_FALSE(pat->try_match(ctx, int_array(2)));
    }

    SECTION("Middle subpattern fails: trailing ones are not tried")
    {
        auto a = mock::Mock_Match_Pattern::make();
        auto b = mock::Mock_Match_Pattern::make();
        auto c = mock::Mock_Match_Pattern::make();

        trompeloeil::sequence seq;
        REQUIRE_CALL(*a, do_try_match(_, _)).IN_SEQUENCE(seq).RETURN(true);
        REQUIRE_CALL(*b, do_try_match(_, _)).IN_SEQUENCE(seq).RETURN(false);
        FORBID_CALL(*c, do_try_match(_, _));

        std::vector<Match_Pattern::Ptr> subs;
        subs.push_back(std::move(a));
        subs.push_back(std::move(b));
        subs.push_back(std::move(c));
        auto pat = make_array(std::move(subs));

        CHECK_FALSE(pat->try_match(ctx, int_array(3)));
    }
}

TEST_CASE("Match_Array: rest clause (named)")
{
    SECTION("Array longer than subpatterns binds the tail")
    {
        Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto a = mock::Mock_Match_Pattern::make();
        REQUIRE_CALL(*a, do_try_match(_, _))
            .WITH(_2->template raw_get<Int>() == 1_f)
            .RETURN(true);

        std::vector<Match_Pattern::Ptr> subs;
        subs.push_back(std::move(a));
        auto pat = make_array(std::move(subs), named_rest("rest"));

        CHECK(pat->try_match(ctx, int_array(4))); // [1, 2, 3, 4]
        REQUIRE(syms.has("rest"));

        auto rest_val = syms.lookup("rest");
        REQUIRE(rest_val->is<Array>());
        const auto& tail = rest_val->raw_get<Array>();
        REQUIRE(tail.size() == 3);
        CHECK(tail[0]->raw_get<Int>() == 2_f);
        CHECK(tail[1]->raw_get<Int>() == 3_f);
        CHECK(tail[2]->raw_get<Int>() == 4_f);
    }

    SECTION("Array exactly as long as subpatterns binds an empty tail")
    {
        Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto a = mock::Mock_Match_Pattern::make();
        auto b = mock::Mock_Match_Pattern::make();
        REQUIRE_CALL(*a, do_try_match(_, _)).RETURN(true);
        REQUIRE_CALL(*b, do_try_match(_, _)).RETURN(true);

        std::vector<Match_Pattern::Ptr> subs;
        subs.push_back(std::move(a));
        subs.push_back(std::move(b));
        auto pat = make_array(std::move(subs), named_rest("rest"));

        CHECK(pat->try_match(ctx, int_array(2)));
        REQUIRE(syms.has("rest"));
        auto rest_val = syms.lookup("rest");
        REQUIRE(rest_val->is<Array>());
        CHECK(rest_val->raw_get<Array>().empty());
    }

    SECTION("Array shorter than subpatterns fails without binding rest")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto a = mock::Mock_Match_Pattern::make();
        auto b = mock::Mock_Match_Pattern::make();
        FORBID_CALL(*a, do_try_match(_, _));
        FORBID_CALL(*b, do_try_match(_, _));
        FORBID_CALL(syms, define(_, _));

        std::vector<Match_Pattern::Ptr> subs;
        subs.push_back(std::move(a));
        subs.push_back(std::move(b));
        auto pat = make_array(std::move(subs), named_rest("rest"));

        CHECK_FALSE(pat->try_match(ctx, int_array(1)));
    }

    SECTION("No subpatterns + rest binds the whole array")
    {
        Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto pat = make_array({}, named_rest("all"));
        CHECK(pat->try_match(ctx, int_array(3)));

        REQUIRE(syms.has("all"));
        auto val = syms.lookup("all");
        REQUIRE(val->is<Array>());
        CHECK(val->raw_get<Array>().size() == 3);
    }

    SECTION("Subpattern failure before rest does not bind rest")
    {
        mock::Mock_Symbol_Table syms;
        Execution_Context ctx{.symbols = syms};

        auto a = mock::Mock_Match_Pattern::make();
        REQUIRE_CALL(*a, do_try_match(_, _)).RETURN(false);
        // define() must never be reached because the match fails.
        FORBID_CALL(syms, define(_, _));

        std::vector<Match_Pattern::Ptr> subs;
        subs.push_back(std::move(a));
        auto pat = make_array(std::move(subs), named_rest("rest"));

        CHECK_FALSE(pat->try_match(ctx, int_array(3)));
    }
}

TEST_CASE("Match_Array: rest clause (discard)")
{
    mock::Mock_Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};

    SECTION("Longer array matches without binding anything")
    {
        auto a = mock::Mock_Match_Pattern::make();
        REQUIRE_CALL(*a, do_try_match(_, _)).RETURN(true);

        // Discarded rest must not call define() for its tail.
        FORBID_CALL(syms, define(_, _));

        std::vector<Match_Pattern::Ptr> subs;
        subs.push_back(std::move(a));
        auto pat = make_array(std::move(subs), discard_rest());

        CHECK(pat->try_match(ctx, int_array(5))); // [1..5], 4 dropped
    }

    SECTION("Exactly-as-long array matches without binding anything")
    {
        auto a = mock::Mock_Match_Pattern::make();
        auto b = mock::Mock_Match_Pattern::make();
        REQUIRE_CALL(*a, do_try_match(_, _)).RETURN(true);
        REQUIRE_CALL(*b, do_try_match(_, _)).RETURN(true);
        FORBID_CALL(syms, define(_, _));

        std::vector<Match_Pattern::Ptr> subs;
        subs.push_back(std::move(a));
        subs.push_back(std::move(b));
        auto pat = make_array(std::move(subs), discard_rest());

        CHECK(pat->try_match(ctx, int_array(2)));
    }

    SECTION("Shorter array fails")
    {
        auto a = mock::Mock_Match_Pattern::make();
        auto b = mock::Mock_Match_Pattern::make();
        FORBID_CALL(*a, do_try_match(_, _));
        FORBID_CALL(*b, do_try_match(_, _));
        FORBID_CALL(syms, define(_, _));

        std::vector<Match_Pattern::Ptr> subs;
        subs.push_back(std::move(a));
        subs.push_back(std::move(b));
        auto pat = make_array(std::move(subs), discard_rest());

        CHECK_FALSE(pat->try_match(ctx, int_array(1)));
    }
}

TEST_CASE("Match_Array: symbol_sequence")
{
    // symbol_sequence walks children (each yields whatever its
    // symbol_sequence produces) and then yields the rest binding if named.
    // Mock sub-patterns inherit the default symbol_sequence that walks
    // children (none) and yields nothing, so we can observe the rest
    // binding cleanly in isolation.

    SECTION("No rest: yields only sub-pattern actions")
    {
        auto a = mock::Mock_Match_Pattern::make();
        auto b = mock::Mock_Match_Pattern::make();

        std::vector<Match_Pattern::Ptr> subs;
        subs.push_back(std::move(a));
        subs.push_back(std::move(b));
        auto pat = make_array(std::move(subs));

        auto actions = pat->symbol_sequence() | std::ranges::to<std::vector>();
        CHECK(actions.empty());
    }

    SECTION("Named rest yields a Definition for the rest name")
    {
        auto pat = make_array({}, named_rest("rest"));
        auto actions = pat->symbol_sequence() | std::ranges::to<std::vector>();
        REQUIRE(actions.size() == 1);
        auto* def = std::get_if<AST_Node::Definition>(&actions[0]);
        REQUIRE(def);
        CHECK(def->name == "rest");
        CHECK(def->exported == false);
    }

    SECTION("Discarded rest yields nothing")
    {
        auto pat = make_array({}, discard_rest());
        auto actions = pat->symbol_sequence() | std::ranges::to<std::vector>();
        CHECK(actions.empty());
    }
}

TEST_CASE("Match_Array: children")
{
    auto a = mock::Mock_Match_Pattern::make();
    auto b = mock::Mock_Match_Pattern::make();
    auto c = mock::Mock_Match_Pattern::make();
    auto* a_raw = a.get();
    auto* b_raw = b.get();
    auto* c_raw = c.get();

    std::vector<Match_Pattern::Ptr> subs;
    subs.push_back(std::move(a));
    subs.push_back(std::move(b));
    subs.push_back(std::move(c));

    SECTION("Children yields all subpatterns in order (no rest)")
    {
        auto pat = make_array(std::move(subs));
        auto kids = pat->children() | std::ranges::to<std::vector>();
        REQUIRE(kids.size() == 3);
        CHECK(kids[0].node == a_raw);
        CHECK(kids[1].node == b_raw);
        CHECK(kids[2].node == c_raw);
    }

    SECTION("Rest clause does not appear in children (not a subtree)")
    {
        auto pat = make_array(std::move(subs), named_rest("rest"));
        auto kids = pat->children() | std::ranges::to<std::vector>();
        CHECK(kids.size() == 3);
    }
}

TEST_CASE("Match_Array: node_label")
{
    SECTION("No rest")
    {
        auto pat = make_array({});
        CHECK(pat->node_label() == "Match_Array");
    }

    SECTION("Named rest")
    {
        auto pat = make_array({}, named_rest("rest"));
        CHECK(pat->node_label() == "Match_Array(...rest)");
    }

    SECTION("Discarded rest")
    {
        auto pat = make_array({}, discard_rest());
        CHECK(pat->node_label() == "Match_Array(..._)");
    }
}
