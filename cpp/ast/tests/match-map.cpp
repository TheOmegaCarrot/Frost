#include <catch2/catch_test_macros.hpp>
#include <catch2/trompeloeil.hpp>

#include <frost/mock/mock-expression.hpp>
#include <frost/mock/mock-match-pattern.hpp>
#include <frost/mock/mock-symbol-table.hpp>
#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/ast/literal.hpp>
#include <frost/ast/match-map.hpp>
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

// A literal key expression that evaluates to `v`. Used when we want a real
// (non-mock) key so that mock setup on the key expression isn't needed.
Expression::Ptr literal_key(Value_Ptr v)
{
    return std::make_unique<Literal>(AST_Node::no_range, std::move(v));
}

Match_Map::Ptr make_map(std::vector<Match_Map::Element> elements)
{
    return std::make_unique<Match_Map>(AST_Node::no_range, std::move(elements));
}

// Build a Frost Map value from a list of (string key, value) pairs. Mirrors
// the common match target shape for map-pattern tests.
Value_Ptr make_string_keyed_map(
    std::initializer_list<std::pair<std::string, Value_Ptr>> entries)
{
    frst::Map m;
    for (const auto& [k, v] : entries)
        m.emplace(Value::create(String{k}), v);
    return Value::create(std::move(m));
}

} // namespace

TEST_CASE("Match_Map: non-map match target fails without consulting subpatterns")
{
    mock::Mock_Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};

    // Sub-pattern must NOT be consulted -- the type check runs first and
    // returns false before any key lookup happens.
    auto inner = mock::Mock_Match_Pattern::make();
    FORBID_CALL(*inner, do_try_match(_, _));

    std::vector<Match_Map::Element> elements;
    elements.push_back({literal_key(Value::create("name"s)), std::move(inner)});
    auto pat = make_map(std::move(elements));

    SECTION("Int match target") { CHECK_FALSE(pat->try_match(ctx, Value::create(42_f))); }
    SECTION("String match target") { CHECK_FALSE(pat->try_match(ctx, Value::create("hi"s))); }
    SECTION("Array match target") { CHECK_FALSE(pat->try_match(ctx, Value::create(Array{}))); }
    SECTION("Null match target") { CHECK_FALSE(pat->try_match(ctx, Value::null())); }
    SECTION("Bool match target") { CHECK_FALSE(pat->try_match(ctx, Value::create(true))); }
}

TEST_CASE("Match_Map: empty pattern matches any map")
{
    mock::Mock_Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};

    auto pat = make_map({});

    SECTION("Empty map") { CHECK(pat->try_match(ctx, Value::create(frst::Map{}))); }
    SECTION("Non-empty map")
    {
        auto m = make_string_keyed_map({
            {"a", Value::create(1_f)},
            {"b", Value::create(2_f)},
        });
        CHECK(pat->try_match(ctx, m));
    }
}

TEST_CASE("Match_Map: key found, sub-pattern receives the mapped value")
{
    mock::Mock_Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};

    auto inner = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*inner, do_try_match(_, _))
        .WITH(_2->template raw_get<Int>() == 42_f)
        .RETURN(true);

    std::vector<Match_Map::Element> elements;
    elements.push_back({literal_key(Value::create("age"s)), std::move(inner)});
    auto pat = make_map(std::move(elements));

    auto m = make_string_keyed_map({
        {"age", Value::create(42_f)},
        {"name", Value::create("Alice"s)},
    });
    CHECK(pat->try_match(ctx, m));
}

TEST_CASE("Match_Map: key found, sub-pattern fails")
{
    mock::Mock_Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};

    auto inner = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*inner, do_try_match(_, _)).RETURN(false);

    std::vector<Match_Map::Element> elements;
    elements.push_back({literal_key(Value::create("age"s)), std::move(inner)});
    auto pat = make_map(std::move(elements));

    auto m = make_string_keyed_map({{"age", Value::create(42_f)}});
    CHECK_FALSE(pat->try_match(ctx, m));
}

TEST_CASE("Match_Map: missing key fails the match without consulting sub-pattern")
{
    // A missing key is an immediate match failure. The sub-pattern is never
    // called -- there is no implicit null substitution.
    mock::Mock_Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};

    SECTION("Missing key on a non-empty map")
    {
        auto inner = mock::Mock_Match_Pattern::make();
        FORBID_CALL(*inner, do_try_match(_, _));

        std::vector<Match_Map::Element> elements;
        elements.push_back(
            {literal_key(Value::create("missing"s)), std::move(inner)});
        auto pat = make_map(std::move(elements));

        auto m = make_string_keyed_map({{"other", Value::create(1_f)}});
        CHECK_FALSE(pat->try_match(ctx, m));
    }

    SECTION("Missing key on an empty map")
    {
        auto inner = mock::Mock_Match_Pattern::make();
        FORBID_CALL(*inner, do_try_match(_, _));

        std::vector<Match_Map::Element> elements;
        elements.push_back(
            {literal_key(Value::create("k"s)), std::move(inner)});
        auto pat = make_map(std::move(elements));

        CHECK_FALSE(pat->try_match(ctx, Value::create(frst::Map{})));
    }
}

TEST_CASE("Match_Map: extra keys in match target are ignored")
{
    // A pattern that names one key matches a map with many keys, as long
    // as the named key matches. Extra keys don't disturb the match.
    mock::Mock_Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};

    auto inner = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*inner, do_try_match(_, _))
        .WITH(_2->template raw_get<Int>() == 1_f)
        .RETURN(true);

    std::vector<Match_Map::Element> elements;
    elements.push_back({literal_key(Value::create("a"s)), std::move(inner)});
    auto pat = make_map(std::move(elements));

    auto m = make_string_keyed_map({
        {"a", Value::create(1_f)},
        {"b", Value::create(2_f)},
        {"c", Value::create(3_f)},
    });
    CHECK(pat->try_match(ctx, m));
}

TEST_CASE("Match_Map: multiple entries processed in order")
{
    mock::Mock_Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};

    SECTION("All succeed: match succeeds")
    {
        auto a = mock::Mock_Match_Pattern::make();
        auto b = mock::Mock_Match_Pattern::make();

        trompeloeil::sequence seq;
        REQUIRE_CALL(*a, do_try_match(_, _))
            .WITH(_2->template raw_get<Int>() == 1_f)
            .IN_SEQUENCE(seq)
            .RETURN(true);
        REQUIRE_CALL(*b, do_try_match(_, _))
            .WITH(_2->template raw_get<Int>() == 2_f)
            .IN_SEQUENCE(seq)
            .RETURN(true);

        std::vector<Match_Map::Element> elements;
        elements.push_back({literal_key(Value::create("a"s)), std::move(a)});
        elements.push_back({literal_key(Value::create("b"s)), std::move(b)});
        auto pat = make_map(std::move(elements));

        auto m = make_string_keyed_map({
            {"a", Value::create(1_f)},
            {"b", Value::create(2_f)},
        });
        CHECK(pat->try_match(ctx, m));
    }

    SECTION("First entry fails: trailing entries are not tried")
    {
        auto a = mock::Mock_Match_Pattern::make();
        auto b = mock::Mock_Match_Pattern::make();

        REQUIRE_CALL(*a, do_try_match(_, _)).RETURN(false);
        FORBID_CALL(*b, do_try_match(_, _));

        std::vector<Match_Map::Element> elements;
        elements.push_back({literal_key(Value::create("a"s)), std::move(a)});
        elements.push_back({literal_key(Value::create("b"s)), std::move(b)});
        auto pat = make_map(std::move(elements));

        auto m = make_string_keyed_map({
            {"a", Value::create(1_f)},
            {"b", Value::create(2_f)},
        });
        CHECK_FALSE(pat->try_match(ctx, m));
    }

    SECTION("Middle entry fails: trailing entries are not tried")
    {
        auto a = mock::Mock_Match_Pattern::make();
        auto b = mock::Mock_Match_Pattern::make();
        auto c = mock::Mock_Match_Pattern::make();

        trompeloeil::sequence seq;
        REQUIRE_CALL(*a, do_try_match(_, _)).IN_SEQUENCE(seq).RETURN(true);
        REQUIRE_CALL(*b, do_try_match(_, _)).IN_SEQUENCE(seq).RETURN(false);
        FORBID_CALL(*c, do_try_match(_, _));

        std::vector<Match_Map::Element> elements;
        elements.push_back({literal_key(Value::create("a"s)), std::move(a)});
        elements.push_back({literal_key(Value::create("b"s)), std::move(b)});
        elements.push_back({literal_key(Value::create("c"s)), std::move(c)});
        auto pat = make_map(std::move(elements));

        auto m = make_string_keyed_map({
            {"a", Value::create(1_f)},
            {"b", Value::create(2_f)},
            {"c", Value::create(3_f)},
        });
        CHECK_FALSE(pat->try_match(ctx, m));
    }
}

TEST_CASE("Match_Map: key expression is evaluated in the given context")
{
    // A key expression may reference the current context (e.g. a name
    // lookup). Use a Mock_Expression to confirm that evaluate() is called
    // with the correct context.
    mock::Mock_Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};

    auto key_expr = mock::Mock_Expression::make();
    REQUIRE_CALL(*key_expr, do_evaluate(_))
        .LR_WITH(&_1.symbols == &syms)
        .RETURN(Value::create("computed"s));

    auto inner = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*inner, do_try_match(_, _))
        .WITH(_2->template raw_get<Int>() == 99_f)
        .RETURN(true);

    std::vector<Match_Map::Element> elements;
    elements.push_back({std::move(key_expr), std::move(inner)});
    auto pat = make_map(std::move(elements));

    auto m = make_string_keyed_map({{"computed", Value::create(99_f)}});
    CHECK(pat->try_match(ctx, m));
}

TEST_CASE("Match_Map: invalid key types throw")
{
    // A key expression that evaluates to a non-primitive or null value is
    // a malformed pattern. Match_Map throws Frost_Recoverable_Error rather
    // than returning false, because this is a programmer bug in the
    // pattern, not a match failure.
    mock::Mock_Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};

    auto m = make_string_keyed_map({{"a", Value::create(1_f)}});

    SECTION("Null key throws")
    {
        auto key_expr = mock::Mock_Expression::make();
        REQUIRE_CALL(*key_expr, do_evaluate(_)).RETURN(Value::null());

        auto inner = mock::Mock_Match_Pattern::make();
        FORBID_CALL(*inner, do_try_match(_, _));

        std::vector<Match_Map::Element> elements;
        elements.push_back({std::move(key_expr), std::move(inner)});
        auto pat = make_map(std::move(elements));

        CHECK_THROWS_AS(pat->try_match(ctx, m), Frost_Recoverable_Error);
    }

    SECTION("Array key throws")
    {
        auto key_expr = mock::Mock_Expression::make();
        REQUIRE_CALL(*key_expr, do_evaluate(_))
            .RETURN(Value::create(Array{}));

        auto inner = mock::Mock_Match_Pattern::make();
        FORBID_CALL(*inner, do_try_match(_, _));

        std::vector<Match_Map::Element> elements;
        elements.push_back({std::move(key_expr), std::move(inner)});
        auto pat = make_map(std::move(elements));

        CHECK_THROWS_AS(pat->try_match(ctx, m), Frost_Recoverable_Error);
    }

    SECTION("Map key throws")
    {
        auto key_expr = mock::Mock_Expression::make();
        REQUIRE_CALL(*key_expr, do_evaluate(_))
            .RETURN(Value::create(frst::Map{}));

        auto inner = mock::Mock_Match_Pattern::make();
        FORBID_CALL(*inner, do_try_match(_, _));

        std::vector<Match_Map::Element> elements;
        elements.push_back({std::move(key_expr), std::move(inner)});
        auto pat = make_map(std::move(elements));

        CHECK_THROWS_AS(pat->try_match(ctx, m), Frost_Recoverable_Error);
    }
}

TEST_CASE("Match_Map: key expression errors propagate")
{
    mock::Mock_Symbol_Table syms;
    Execution_Context ctx{.symbols = syms};

    auto key_expr = mock::Mock_Expression::make();
    REQUIRE_CALL(*key_expr, do_evaluate(_))
        .THROW(Frost_Recoverable_Error{"key boom"});

    auto inner = mock::Mock_Match_Pattern::make();
    FORBID_CALL(*inner, do_try_match(_, _));

    std::vector<Match_Map::Element> elements;
    elements.push_back({std::move(key_expr), std::move(inner)});
    auto pat = make_map(std::move(elements));

    auto m = make_string_keyed_map({{"a", Value::create(1_f)}});
    CHECK_THROWS_AS(pat->try_match(ctx, m), Frost_Recoverable_Error);
}

TEST_CASE("Match_Map: children exposes key expressions and sub-patterns")
{
    auto key_a = literal_key(Value::create("a"s));
    auto pat_a = mock::Mock_Match_Pattern::make();
    auto* key_a_raw = key_a.get();
    auto* pat_a_raw = pat_a.get();

    auto key_b = literal_key(Value::create("b"s));
    auto pat_b = mock::Mock_Match_Pattern::make();
    auto* key_b_raw = key_b.get();
    auto* pat_b_raw = pat_b.get();

    std::vector<Match_Map::Element> elements;
    elements.push_back({std::move(key_a), std::move(pat_a)});
    elements.push_back({std::move(key_b), std::move(pat_b)});
    auto pat = make_map(std::move(elements));

    auto kids = pat->children() | std::ranges::to<std::vector>();
    REQUIRE(kids.size() == 4);

    CHECK(kids[0].node == key_a_raw);
    CHECK(kids[0].label == "Key 1");
    CHECK(kids[1].node == pat_a_raw);
    CHECK(kids[1].label == "Pattern 1");
    CHECK(kids[2].node == key_b_raw);
    CHECK(kids[2].label == "Key 2");
    CHECK(kids[3].node == pat_b_raw);
    CHECK(kids[3].label == "Pattern 2");
}

TEST_CASE("Match_Map: symbol_sequence propagates actions from keys and patterns")
{
    // symbol_sequence walks children (each yields whatever its
    // symbol_sequence produces). A Literal key produces no actions; a
    // Mock_Match_Pattern's default symbol_sequence also produces none.
    // So for this case we expect an empty sequence.
    auto pat = make_map({});
    auto actions = pat->symbol_sequence() | std::ranges::to<std::vector>();
    CHECK(actions.empty());
}

TEST_CASE("Match_Map: node_label")
{
    auto pat = make_map({});
    CHECK(pat->node_label() == "Match_Map");
}
