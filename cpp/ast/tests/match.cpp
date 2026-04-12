#include <catch2/catch_test_macros.hpp>
#include <catch2/trompeloeil.hpp>

#include <frost/mock/mock-expression.hpp>
#include <frost/mock/mock-match-pattern.hpp>
#include <frost/mock/mock-symbol-table.hpp>
#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/ast/match.hpp>
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

// Convenience: build a Match node from a target and a list of arms.
Match::Ptr make_match(Expression::Ptr target, std::vector<Match::Arm> arms)
{
    return std::make_unique<Match>(AST_Node::no_range, std::move(target),
                                   std::move(arms));
}

// Convenience: build an Arm without a guard.
Match::Arm arm(Match_Pattern::Ptr pat, Expression::Ptr result)
{
    return Match::Arm{.pattern = std::move(pat),
                      .guard = std::nullopt,
                      .result = std::move(result)};
}

// Convenience: build an Arm with a guard.
Match::Arm guarded_arm(Match_Pattern::Ptr pat, Expression::Ptr guard,
                       Expression::Ptr result)
{
    return Match::Arm{.pattern = std::move(pat),
                      .guard = std::move(guard),
                      .result = std::move(result)};
}

} // namespace

// =============================================================================
// do_evaluate
// =============================================================================

TEST_CASE("Match: evaluates target exactly once")
{
    Symbol_Table syms;
    Evaluation_Context ctx{.symbols = syms};

    auto target = mock::Mock_Expression::make();
    REQUIRE_CALL(*target, do_evaluate(_))
        .TIMES(1)
        .RETURN(Value::create(42_f));

    auto pat = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*pat, do_try_match(_, _)).RETURN(true);

    auto result = mock::Mock_Expression::make();
    REQUIRE_CALL(*result, do_evaluate(_)).RETURN(Value::create("result"s));

    std::vector<Match::Arm> arms;
    arms.push_back(arm(std::move(pat), std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    auto out = m->evaluate(ctx);
    REQUIRE(out->is<String>());
    CHECK(out->raw_get<String>() == "result");
}

TEST_CASE("Match: target result is passed to the pattern unchanged")
{
    // The evaluated target value is what the pattern must receive. This
    // verifies the plumbing from target evaluation into pattern dispatch.
    Symbol_Table syms;
    Evaluation_Context ctx{.symbols = syms};

    auto target_value = Value::create(1234_f);

    auto target = mock::Mock_Expression::make();
    REQUIRE_CALL(*target, do_evaluate(_)).RETURN(target_value);

    auto pat = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*pat, do_try_match(_, _))
        .LR_WITH(_2.get() == target_value.get())
        .RETURN(true);

    auto result = mock::Mock_Expression::make();
    REQUIRE_CALL(*result, do_evaluate(_)).RETURN(Value::null());

    std::vector<Match::Arm> arms;
    arms.push_back(arm(std::move(pat), std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    auto _ = m->evaluate(ctx);
}

TEST_CASE("Match: returns the matching arm's result")
{
    Symbol_Table syms;
    Evaluation_Context ctx{.symbols = syms};

    auto target = mock::Mock_Expression::make();
    REQUIRE_CALL(*target, do_evaluate(_)).RETURN(Value::create(1_f));

    auto pat = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*pat, do_try_match(_, _)).RETURN(true);

    auto result = mock::Mock_Expression::make();
    REQUIRE_CALL(*result, do_evaluate(_))
        .RETURN(Value::create("chosen result"s));

    std::vector<Match::Arm> arms;
    arms.push_back(arm(std::move(pat), std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    auto out = m->evaluate(ctx);
    REQUIRE(out->is<String>());
    CHECK(out->raw_get<String>() == "chosen result");
}

TEST_CASE("Match: no matching arm throws")
{
    Symbol_Table syms;
    Evaluation_Context ctx{.symbols = syms};

    auto target = mock::Mock_Expression::make();
    REQUIRE_CALL(*target, do_evaluate(_)).RETURN(Value::create(1_f));

    auto pat1 = mock::Mock_Match_Pattern::make();
    auto pat2 = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*pat1, do_try_match(_, _)).RETURN(false);
    REQUIRE_CALL(*pat2, do_try_match(_, _)).RETURN(false);

    auto result1 = mock::Mock_Expression::make();
    auto result2 = mock::Mock_Expression::make();
    FORBID_CALL(*result1, do_evaluate(_));
    FORBID_CALL(*result2, do_evaluate(_));

    std::vector<Match::Arm> arms;
    arms.push_back(arm(std::move(pat1), std::move(result1)));
    arms.push_back(arm(std::move(pat2), std::move(result2)));
    auto m = make_match(std::move(target), std::move(arms));

    CHECK_THROWS(m->evaluate(ctx));
}

TEST_CASE("Match: empty arm list throws after evaluating target")
{
    // Match with no arms evaluates the target then throws because there
    // are zero arms to try.
    Symbol_Table syms;
    Evaluation_Context ctx{.symbols = syms};

    auto target = mock::Mock_Expression::make();
    REQUIRE_CALL(*target, do_evaluate(_))
        .TIMES(1)
        .RETURN(Value::create(1_f));

    auto m = make_match(std::move(target), {});
    CHECK_THROWS(m->evaluate(ctx));
}

TEST_CASE("Match: first-match-wins short-circuits later arms")
{
    Symbol_Table syms;
    Evaluation_Context ctx{.symbols = syms};

    auto target = mock::Mock_Expression::make();
    REQUIRE_CALL(*target, do_evaluate(_)).RETURN(Value::create(1_f));

    // Arm 1: pattern fails
    auto pat1 = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*pat1, do_try_match(_, _)).RETURN(false);
    auto result1 = mock::Mock_Expression::make();
    FORBID_CALL(*result1, do_evaluate(_));

    // Arm 2: pattern succeeds (this is the winner)
    auto pat2 = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*pat2, do_try_match(_, _)).RETURN(true);
    auto result2 = mock::Mock_Expression::make();
    REQUIRE_CALL(*result2, do_evaluate(_))
        .RETURN(Value::create("second"s));

    // Arm 3: pattern should never be consulted
    auto pat3 = mock::Mock_Match_Pattern::make();
    FORBID_CALL(*pat3, do_try_match(_, _));
    auto result3 = mock::Mock_Expression::make();
    FORBID_CALL(*result3, do_evaluate(_));

    std::vector<Match::Arm> arms;
    arms.push_back(arm(std::move(pat1), std::move(result1)));
    arms.push_back(arm(std::move(pat2), std::move(result2)));
    arms.push_back(arm(std::move(pat3), std::move(result3)));
    auto m = make_match(std::move(target), std::move(arms));

    auto out = m->evaluate(ctx);
    REQUIRE(out->is<String>());
    CHECK(out->raw_get<String>() == "second");
}

TEST_CASE("Match: arms are tried in declaration order")
{
    Symbol_Table syms;
    Evaluation_Context ctx{.symbols = syms};

    auto target = mock::Mock_Expression::make();
    REQUIRE_CALL(*target, do_evaluate(_)).RETURN(Value::create(1_f));

    trompeloeil::sequence seq;

    auto pat1 = mock::Mock_Match_Pattern::make();
    auto pat2 = mock::Mock_Match_Pattern::make();
    auto pat3 = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*pat1, do_try_match(_, _))
        .IN_SEQUENCE(seq)
        .RETURN(false);
    REQUIRE_CALL(*pat2, do_try_match(_, _))
        .IN_SEQUENCE(seq)
        .RETURN(false);
    REQUIRE_CALL(*pat3, do_try_match(_, _))
        .IN_SEQUENCE(seq)
        .RETURN(false);

    auto r1 = mock::Mock_Expression::make();
    auto r2 = mock::Mock_Expression::make();
    auto r3 = mock::Mock_Expression::make();
    FORBID_CALL(*r1, do_evaluate(_));
    FORBID_CALL(*r2, do_evaluate(_));
    FORBID_CALL(*r3, do_evaluate(_));

    std::vector<Match::Arm> arms;
    arms.push_back(arm(std::move(pat1), std::move(r1)));
    arms.push_back(arm(std::move(pat2), std::move(r2)));
    arms.push_back(arm(std::move(pat3), std::move(r3)));
    auto m = make_match(std::move(target), std::move(arms));

    CHECK_THROWS(m->evaluate(ctx));
}

// =============================================================================
// Guards
// =============================================================================

TEST_CASE("Match: guard runs only after its pattern matches")
{
    Symbol_Table syms;
    Evaluation_Context ctx{.symbols = syms};

    auto target = mock::Mock_Expression::make();
    REQUIRE_CALL(*target, do_evaluate(_)).RETURN(Value::create(1_f));

    // Pattern fails: guard must not run.
    auto pat = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*pat, do_try_match(_, _)).RETURN(false);

    auto guard = mock::Mock_Expression::make();
    FORBID_CALL(*guard, do_evaluate(_));

    auto result = mock::Mock_Expression::make();
    FORBID_CALL(*result, do_evaluate(_));

    std::vector<Match::Arm> arms;
    arms.push_back(guarded_arm(std::move(pat), std::move(guard),
                               std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    CHECK_THROWS(m->evaluate(ctx));
}

TEST_CASE("Match: truthy guard allows arm to win")
{
    Symbol_Table syms;
    Evaluation_Context ctx{.symbols = syms};

    auto target = mock::Mock_Expression::make();
    REQUIRE_CALL(*target, do_evaluate(_)).RETURN(Value::create(1_f));

    auto pat = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*pat, do_try_match(_, _)).RETURN(true);

    auto guard = mock::Mock_Expression::make();
    REQUIRE_CALL(*guard, do_evaluate(_)).RETURN(Value::create(true));

    auto result = mock::Mock_Expression::make();
    REQUIRE_CALL(*result, do_evaluate(_))
        .RETURN(Value::create("guard passed"s));

    std::vector<Match::Arm> arms;
    arms.push_back(guarded_arm(std::move(pat), std::move(guard),
                               std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    auto out = m->evaluate(ctx);
    REQUIRE(out->is<String>());
    CHECK(out->raw_get<String>() == "guard passed");
}

TEST_CASE("Match: non-truthy guard skips the arm")
{
    Symbol_Table syms;
    Evaluation_Context ctx{.symbols = syms};

    auto target = mock::Mock_Expression::make();
    REQUIRE_CALL(*target, do_evaluate(_)).RETURN(Value::create(1_f));

    // Arm 1: pattern succeeds but guard is falsy -> skip
    auto pat1 = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*pat1, do_try_match(_, _)).RETURN(true);
    auto guard1 = mock::Mock_Expression::make();
    REQUIRE_CALL(*guard1, do_evaluate(_)).RETURN(Value::create(false));
    auto result1 = mock::Mock_Expression::make();
    FORBID_CALL(*result1, do_evaluate(_));

    // Arm 2: pattern succeeds, no guard -> winner
    auto pat2 = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*pat2, do_try_match(_, _)).RETURN(true);
    auto result2 = mock::Mock_Expression::make();
    REQUIRE_CALL(*result2, do_evaluate(_))
        .RETURN(Value::create("second"s));

    std::vector<Match::Arm> arms;
    arms.push_back(guarded_arm(std::move(pat1), std::move(guard1),
                               std::move(result1)));
    arms.push_back(arm(std::move(pat2), std::move(result2)));
    auto m = make_match(std::move(target), std::move(arms));

    auto out = m->evaluate(ctx);
    REQUIRE(out->is<String>());
    CHECK(out->raw_get<String>() == "second");
}

TEST_CASE("Match: guard that returns null is falsy (arm skipped)")
{
    // In Frost, null is falsy. A guard returning null should behave like
    // a false guard -- the arm is skipped and no-match throws.
    Symbol_Table syms;
    Evaluation_Context ctx{.symbols = syms};

    auto target = mock::Mock_Expression::make();
    REQUIRE_CALL(*target, do_evaluate(_)).RETURN(Value::create(1_f));

    auto pat = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*pat, do_try_match(_, _)).RETURN(true);

    auto guard = mock::Mock_Expression::make();
    REQUIRE_CALL(*guard, do_evaluate(_)).RETURN(Value::null());

    auto result = mock::Mock_Expression::make();
    FORBID_CALL(*result, do_evaluate(_));

    std::vector<Match::Arm> arms;
    arms.push_back(guarded_arm(std::move(pat), std::move(guard),
                               std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    CHECK_THROWS(m->evaluate(ctx));
}

// =============================================================================
// Scoping: outer visibility, pattern bindings, per-arm isolation
// =============================================================================

TEST_CASE("Match: target is evaluated in the outer context, not an arm context")
{
    // The target expression must be evaluated against the caller's
    // Symbol_Table directly, not against an arm-scratch table. A bug that
    // moved target evaluation into the arm loop would re-evaluate per arm
    // and could observe shadowed names from arm scopes -- this test pins
    // the target's `_1.symbols` to the outer table by address.
    Symbol_Table syms;
    Evaluation_Context ctx{.symbols = syms};

    auto target = mock::Mock_Expression::make();
    REQUIRE_CALL(*target, do_evaluate(_))
        .LR_WITH(&_1.symbols == &syms)
        .RETURN(Value::create(1_f));

    auto pat = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*pat, do_try_match(_, _)).RETURN(true);

    auto result = mock::Mock_Expression::make();
    REQUIRE_CALL(*result, do_evaluate(_)).RETURN(Value::null());

    std::vector<Match::Arm> arms;
    arms.push_back(arm(std::move(pat), std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    auto _ = m->evaluate(ctx);
}

TEST_CASE("Match: pattern receives an arm context distinct from the outer table")
{
    // The context handed to the pattern must (a) provide the outer
    // failover (so the pattern can read outer names) but (b) NOT be the
    // outer table itself (so writes from the pattern don't leak out).
    Symbol_Table syms;
    syms.define("outer_var", Value::create(1_f));
    Evaluation_Context ctx{.symbols = syms};

    auto target = mock::Mock_Expression::make();
    REQUIRE_CALL(*target, do_evaluate(_)).RETURN(Value::create(1_f));

    auto pat = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*pat, do_try_match(_, _))
        .LR_WITH(&_1.symbols != &syms)
        .WITH(_1.symbols.has("outer_var"))
        .RETURN(true);

    auto result = mock::Mock_Expression::make();
    REQUIRE_CALL(*result, do_evaluate(_)).RETURN(Value::null());

    std::vector<Match::Arm> arms;
    arms.push_back(arm(std::move(pat), std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    auto _ = m->evaluate(ctx);
}

TEST_CASE("Match: guard's eval context has both pattern bindings and outer failover")
{
    // Direct check on the guard's `_1`: the same context must contain
    // pattern-locally-defined names AND fall through to outer names. This
    // pins down both halves of the failover chain at once.
    Symbol_Table syms;
    syms.define("outer_var", Value::create(0_f));
    Evaluation_Context ctx{.symbols = syms};

    auto target = mock::Mock_Expression::make();
    REQUIRE_CALL(*target, do_evaluate(_)).RETURN(Value::create(1_f));

    auto pat = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*pat, do_try_match(_, _))
        .SIDE_EFFECT(_1.symbols.define("local", Value::create(0_f)))
        .RETURN(true);

    auto guard = mock::Mock_Expression::make();
    REQUIRE_CALL(*guard, do_evaluate(_))
        .WITH(_1.symbols.has("local") && _1.symbols.has("outer_var"))
        .RETURN(Value::create(true));

    auto result = mock::Mock_Expression::make();
    REQUIRE_CALL(*result, do_evaluate(_)).RETURN(Value::null());

    std::vector<Match::Arm> arms;
    arms.push_back(guarded_arm(std::move(pat), std::move(guard),
                               std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    auto _ = m->evaluate(ctx);
}

TEST_CASE("Match: pattern bindings shadow outer names of the same spelling")
{
    // Outer scope defines `x = 1`. Pattern binds `x = 99` into the arm
    // scope. The result reads `x` and must see the shadowed (pattern-bound)
    // value, NOT the outer one. After the match completes, the outer `x`
    // must be unchanged.
    Symbol_Table syms;
    syms.define("x", Value::create(1_f));
    Evaluation_Context ctx{.symbols = syms};

    auto target = mock::Mock_Expression::make();
    REQUIRE_CALL(*target, do_evaluate(_)).RETURN(Value::create(0_f));

    auto pat = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*pat, do_try_match(_, _))
        .SIDE_EFFECT(_1.symbols.define("x", Value::create(99_f)))
        .RETURN(true);

    auto result = mock::Mock_Expression::make();
    REQUIRE_CALL(*result, do_evaluate(_))
        .LR_RETURN(_1.symbols.lookup("x"));

    std::vector<Match::Arm> arms;
    arms.push_back(arm(std::move(pat), std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    auto out = m->evaluate(ctx);
    REQUIRE(out->is<Int>());
    CHECK(out->raw_get<Int>() == 99_f);

    // Outer `x` is undisturbed by the shadowing.
    auto outer_x = syms.lookup("x");
    REQUIRE(outer_x->is<Int>());
    CHECK(outer_x->raw_get<Int>() == 1_f);
}

TEST_CASE("Match: guard and result see outer context via failover")
{
    // A real Symbol_Table is used here specifically because the test is
    // about failover chain visibility -- we need arm_ctx to look up names
    // from the enclosing scope.
    Symbol_Table syms;
    syms.define("outer_var", Value::create(99_f));
    Evaluation_Context ctx{.symbols = syms};

    auto target = mock::Mock_Expression::make();
    REQUIRE_CALL(*target, do_evaluate(_)).RETURN(Value::create(1_f));

    auto pat = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*pat, do_try_match(_, _))
        .WITH(_1.symbols.has("outer_var"))
        .RETURN(true);

    auto guard = mock::Mock_Expression::make();
    REQUIRE_CALL(*guard, do_evaluate(_))
        .WITH(_1.symbols.has("outer_var"))
        .RETURN(Value::create(true));

    auto result = mock::Mock_Expression::make();
    REQUIRE_CALL(*result, do_evaluate(_))
        .WITH(_1.symbols.has("outer_var"))
        .LR_RETURN(_1.symbols.lookup("outer_var"));

    std::vector<Match::Arm> arms;
    arms.push_back(guarded_arm(std::move(pat), std::move(guard),
                               std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    auto out = m->evaluate(ctx);
    REQUIRE(out->is<Int>());
    CHECK(out->raw_get<Int>() == 99_f);
}

TEST_CASE("Match: pattern bindings are visible to guard and result")
{
    Symbol_Table syms;
    Evaluation_Context ctx{.symbols = syms};

    auto target = mock::Mock_Expression::make();
    REQUIRE_CALL(*target, do_evaluate(_)).RETURN(Value::create(1_f));

    // Pattern binds "bound" into the arm scope as a side effect.
    auto pat = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*pat, do_try_match(_, _))
        .SIDE_EFFECT(_1.symbols.define("bound", Value::create(7_f)))
        .RETURN(true);

    // Guard must see "bound".
    auto guard = mock::Mock_Expression::make();
    REQUIRE_CALL(*guard, do_evaluate(_))
        .WITH(_1.symbols.has("bound"))
        .RETURN(Value::create(true));

    // Result must also see "bound" and return its value.
    auto result = mock::Mock_Expression::make();
    REQUIRE_CALL(*result, do_evaluate(_))
        .WITH(_1.symbols.has("bound"))
        .LR_RETURN(_1.symbols.lookup("bound"));

    std::vector<Match::Arm> arms;
    arms.push_back(guarded_arm(std::move(pat), std::move(guard),
                               std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    auto out = m->evaluate(ctx);
    REQUIRE(out->is<Int>());
    CHECK(out->raw_get<Int>() == 7_f);
}

TEST_CASE("Match: pattern bindings do not leak to the outer context")
{
    // After the match finishes (whether or not an arm matched), the outer
    // symbol table must be unchanged by any pattern binding. This confirms
    // that arm_table's bindings are truly confined to the arm's scope.
    Symbol_Table syms;
    Evaluation_Context ctx{.symbols = syms};

    auto target = mock::Mock_Expression::make();
    REQUIRE_CALL(*target, do_evaluate(_)).RETURN(Value::create(1_f));

    auto pat = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*pat, do_try_match(_, _))
        .SIDE_EFFECT(_1.symbols.define("leaked", Value::create(42_f)))
        .RETURN(true);

    auto result = mock::Mock_Expression::make();
    REQUIRE_CALL(*result, do_evaluate(_)).RETURN(Value::create(0_f));

    std::vector<Match::Arm> arms;
    arms.push_back(arm(std::move(pat), std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    auto _ = m->evaluate(ctx);

    CHECK_FALSE(syms.has("leaked"));
}

TEST_CASE("Match: arm scopes are isolated from each other")
{
    // Arm 1 binds a name and then its guard fails. Arm 2's pattern must
    // NOT see the name Arm 1 bound -- each arm gets a fresh scratch scope.
    Symbol_Table syms;
    Evaluation_Context ctx{.symbols = syms};

    auto target = mock::Mock_Expression::make();
    REQUIRE_CALL(*target, do_evaluate(_)).RETURN(Value::create(1_f));

    auto pat1 = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*pat1, do_try_match(_, _))
        .SIDE_EFFECT(_1.symbols.define("arm1_only", Value::create(1_f)))
        .RETURN(true);

    auto guard1 = mock::Mock_Expression::make();
    REQUIRE_CALL(*guard1, do_evaluate(_)).RETURN(Value::create(false));

    auto result1 = mock::Mock_Expression::make();
    FORBID_CALL(*result1, do_evaluate(_));

    // Arm 2's pattern asserts that arm 1's binding is NOT visible.
    auto pat2 = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*pat2, do_try_match(_, _))
        .WITH(not _1.symbols.has("arm1_only"))
        .RETURN(true);

    auto result2 = mock::Mock_Expression::make();
    REQUIRE_CALL(*result2, do_evaluate(_))
        .WITH(not _1.symbols.has("arm1_only"))
        .RETURN(Value::create("clean"s));

    std::vector<Match::Arm> arms;
    arms.push_back(guarded_arm(std::move(pat1), std::move(guard1),
                               std::move(result1)));
    arms.push_back(arm(std::move(pat2), std::move(result2)));
    auto m = make_match(std::move(target), std::move(arms));

    auto out = m->evaluate(ctx);
    REQUIRE(out->is<String>());
    CHECK(out->raw_get<String>() == "clean");
}

TEST_CASE("Match: failed pattern doesn't leak partial bindings to next arm")
{
    // Even when a pattern itself fails mid-way after partially binding
    // into arm_table, the arm_table dies at the end of the iteration.
    // Arm 2 must see a clean scope.
    Symbol_Table syms;
    Evaluation_Context ctx{.symbols = syms};

    auto target = mock::Mock_Expression::make();
    REQUIRE_CALL(*target, do_evaluate(_)).RETURN(Value::create(1_f));

    // Arm 1: pattern performs a partial bind, then returns false.
    auto pat1 = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*pat1, do_try_match(_, _))
        .SIDE_EFFECT(_1.symbols.define("partial", Value::create(1_f)))
        .RETURN(false);
    auto result1 = mock::Mock_Expression::make();
    FORBID_CALL(*result1, do_evaluate(_));

    // Arm 2 must not see "partial".
    auto pat2 = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*pat2, do_try_match(_, _))
        .WITH(not _1.symbols.has("partial"))
        .RETURN(true);
    auto result2 = mock::Mock_Expression::make();
    REQUIRE_CALL(*result2, do_evaluate(_)).RETURN(Value::create("ok"s));

    std::vector<Match::Arm> arms;
    arms.push_back(arm(std::move(pat1), std::move(result1)));
    arms.push_back(arm(std::move(pat2), std::move(result2)));
    auto m = make_match(std::move(target), std::move(arms));

    auto out = m->evaluate(ctx);
    REQUIRE(out->is<String>());
    CHECK(out->raw_get<String>() == "ok");
}

// =============================================================================
// Error propagation
// =============================================================================

TEST_CASE("Match: target evaluation errors propagate")
{
    Symbol_Table syms;
    Evaluation_Context ctx{.symbols = syms};

    auto target = mock::Mock_Expression::make();
    REQUIRE_CALL(*target, do_evaluate(_))
        .THROW(Frost_Recoverable_Error{"target boom"});

    // Pattern, guard, result should never be consulted.
    auto pat = mock::Mock_Match_Pattern::make();
    FORBID_CALL(*pat, do_try_match(_, _));

    auto result = mock::Mock_Expression::make();
    FORBID_CALL(*result, do_evaluate(_));

    std::vector<Match::Arm> arms;
    arms.push_back(arm(std::move(pat), std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    CHECK_THROWS_AS(m->evaluate(ctx), Frost_Recoverable_Error);
}

TEST_CASE("Match: errors thrown from a pattern's try_match propagate")
{
    // A pattern may throw (e.g. Match_Map with an invalid key expression,
    // or Match_Binding detecting a duplicate definition). These are not
    // treated as "match failure" -- they propagate out.
    Symbol_Table syms;
    Evaluation_Context ctx{.symbols = syms};

    auto target = mock::Mock_Expression::make();
    REQUIRE_CALL(*target, do_evaluate(_)).RETURN(Value::create(1_f));

    auto pat = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*pat, do_try_match(_, _))
        .THROW(Frost_Recoverable_Error{"pattern boom"});

    auto result = mock::Mock_Expression::make();
    FORBID_CALL(*result, do_evaluate(_));

    std::vector<Match::Arm> arms;
    arms.push_back(arm(std::move(pat), std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    CHECK_THROWS_AS(m->evaluate(ctx), Frost_Recoverable_Error);
}

TEST_CASE("Match: errors thrown from a guard propagate")
{
    Symbol_Table syms;
    Evaluation_Context ctx{.symbols = syms};

    auto target = mock::Mock_Expression::make();
    REQUIRE_CALL(*target, do_evaluate(_)).RETURN(Value::create(1_f));

    auto pat = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*pat, do_try_match(_, _)).RETURN(true);

    auto guard = mock::Mock_Expression::make();
    REQUIRE_CALL(*guard, do_evaluate(_))
        .THROW(Frost_Recoverable_Error{"guard boom"});

    auto result = mock::Mock_Expression::make();
    FORBID_CALL(*result, do_evaluate(_));

    std::vector<Match::Arm> arms;
    arms.push_back(guarded_arm(std::move(pat), std::move(guard),
                               std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    CHECK_THROWS_AS(m->evaluate(ctx), Frost_Recoverable_Error);
}

TEST_CASE("Match: errors thrown from a result propagate")
{
    Symbol_Table syms;
    Evaluation_Context ctx{.symbols = syms};

    auto target = mock::Mock_Expression::make();
    REQUIRE_CALL(*target, do_evaluate(_)).RETURN(Value::create(1_f));

    auto pat = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*pat, do_try_match(_, _)).RETURN(true);

    auto result = mock::Mock_Expression::make();
    REQUIRE_CALL(*result, do_evaluate(_))
        .THROW(Frost_Recoverable_Error{"result boom"});

    std::vector<Match::Arm> arms;
    arms.push_back(arm(std::move(pat), std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    CHECK_THROWS_AS(m->evaluate(ctx), Frost_Recoverable_Error);
}

TEST_CASE("Match: duplicate-binding errors from patterns propagate")
{
    // A pattern writing a duplicate binding throws Frost_Unrecoverable_Error
    // (from Symbol_Table::define). Match must let this propagate, not
    // swallow it as a failed match.
    Symbol_Table syms;
    Evaluation_Context ctx{.symbols = syms};

    auto target = mock::Mock_Expression::make();
    REQUIRE_CALL(*target, do_evaluate(_)).RETURN(Value::create(1_f));

    auto pat = mock::Mock_Match_Pattern::make();
    REQUIRE_CALL(*pat, do_try_match(_, _))
        .SIDE_EFFECT(_1.symbols.define("x", Value::create(1_f)))
        .SIDE_EFFECT(_1.symbols.define("x", Value::create(2_f)))
        // The second define throws before this RETURN is reached, but
        // trompeloeil still requires the expectation to be well-formed.
        .RETURN(true);

    auto result = mock::Mock_Expression::make();
    FORBID_CALL(*result, do_evaluate(_));

    std::vector<Match::Arm> arms;
    arms.push_back(arm(std::move(pat), std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    CHECK_THROWS_AS(m->evaluate(ctx), Frost_Unrecoverable_Error);
}

// =============================================================================
// symbol_sequence
// =============================================================================

TEST_CASE("Match: symbol_sequence yields target usages")
{
    // A real Name_Lookup is used here (not a mock) because Mock_Expression
    // produces no symbol_sequence actions; we need a concrete expression
    // that yields a Usage.
    auto target = std::make_unique<Name_Lookup>(AST_Node::no_range, "v"s);

    auto pat = mock::Mock_Match_Pattern::make();
    auto result = mock::Mock_Expression::make();

    std::vector<Match::Arm> arms;
    arms.push_back(arm(std::move(pat), std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    auto actions = m->symbol_sequence() | std::ranges::to<std::vector>();
    REQUIRE(actions.size() == 1);
    auto* use = std::get_if<AST_Node::Usage>(&actions[0]);
    REQUIRE(use);
    CHECK(use->name == "v");
}

TEST_CASE("Match: pattern definitions suppress result usages of the same name")
{
    // Pattern yields def:x; result yields use:x. Within the same arm, the
    // result's use:x must be suppressed because the pattern defined x.
    // Neither the def nor the use should escape as an outer action -- the
    // Match sym_seq yields nothing.
    auto target = mock::Mock_Expression::make();

    // Real Match_Binding: yields def:x
    auto pat = std::make_unique<Match_Binding>(
        AST_Node::no_range, std::optional<std::string>{"x"}, std::nullopt);

    // Real Name_Lookup for "x": yields use:x
    auto result = std::make_unique<Name_Lookup>(AST_Node::no_range, "x"s);

    std::vector<Match::Arm> arms;
    arms.push_back(arm(std::move(pat), std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    auto actions = m->symbol_sequence() | std::ranges::to<std::vector>();
    CHECK(actions.empty());
}

TEST_CASE("Match: pattern definitions do not escape as outer actions")
{
    // Even without any uses, a pattern's definitions are scope-local: they
    // must never leak out of the Match's symbol_sequence.
    auto target = mock::Mock_Expression::make();

    auto pat = std::make_unique<Match_Binding>(
        AST_Node::no_range, std::optional<std::string>{"scope_local"},
        std::nullopt);

    auto result = mock::Mock_Expression::make();

    std::vector<Match::Arm> arms;
    arms.push_back(arm(std::move(pat), std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    auto actions = m->symbol_sequence() | std::ranges::to<std::vector>();
    CHECK(actions.empty());
}

TEST_CASE("Match: result usages of outer names are yielded through")
{
    auto target = mock::Mock_Expression::make();

    auto pat = std::make_unique<Match_Binding>(
        AST_Node::no_range, std::optional<std::string>{"x"}, std::nullopt);

    auto result = std::make_unique<Name_Lookup>(AST_Node::no_range, "outer"s);

    std::vector<Match::Arm> arms;
    arms.push_back(arm(std::move(pat), std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    auto actions = m->symbol_sequence() | std::ranges::to<std::vector>();
    REQUIRE(actions.size() == 1);
    auto* use = std::get_if<AST_Node::Usage>(&actions[0]);
    REQUIRE(use);
    CHECK(use->name == "outer");
}

TEST_CASE("Match: guard usages participate in pattern suppression")
{
    // Pattern binds x; guard uses x and outer_g. The x usage should be
    // suppressed; outer_g should be yielded.
    auto target = mock::Mock_Expression::make();

    auto pat = std::make_unique<Match_Binding>(
        AST_Node::no_range, std::optional<std::string>{"x"}, std::nullopt);

    // Guard: (x + outer_g)
    auto guard = std::make_unique<Binop>(
        AST_Node::no_range,
        std::make_unique<Name_Lookup>(AST_Node::no_range, "x"s),
        Binary_Op::PLUS,
        std::make_unique<Name_Lookup>(AST_Node::no_range, "outer_g"s));

    auto result = mock::Mock_Expression::make();

    std::vector<Match::Arm> arms;
    arms.push_back(guarded_arm(std::move(pat), std::move(guard),
                               std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    auto actions = m->symbol_sequence() | std::ranges::to<std::vector>();
    REQUIRE(actions.size() == 1);
    auto* use = std::get_if<AST_Node::Usage>(&actions[0]);
    REQUIRE(use);
    CHECK(use->name == "outer_g");
}

TEST_CASE("Match: per-arm suppression does not leak across arms")
{
    // Arm 1 binds x and its result uses x -- x should be fully suppressed
    // within arm 1. Arm 2 has no binding but its result uses x --
    // arm 2's use:x is NOT in arm 2's defns, so it must be yielded as an
    // outer usage.
    auto target = mock::Mock_Expression::make();

    // Arm 1: bind x, result uses x (suppressed within arm 1)
    auto pat1 = std::make_unique<Match_Binding>(
        AST_Node::no_range, std::optional<std::string>{"x"}, std::nullopt);
    auto result1 = std::make_unique<Name_Lookup>(AST_Node::no_range, "x"s);

    // Arm 2: no binding, result uses x (this x is outer in arm 2's scope)
    auto pat2 = std::make_unique<Match_Binding>(
        AST_Node::no_range, std::optional<std::string>{"y"}, std::nullopt);
    auto result2 = std::make_unique<Name_Lookup>(AST_Node::no_range, "x"s);

    std::vector<Match::Arm> arms;
    arms.push_back(arm(std::move(pat1), std::move(result1)));
    arms.push_back(arm(std::move(pat2), std::move(result2)));
    auto m = make_match(std::move(target), std::move(arms));

    auto actions = m->symbol_sequence() | std::ranges::to<std::vector>();
    // Only arm 2's use:x should escape.
    REQUIRE(actions.size() == 1);
    auto* use = std::get_if<AST_Node::Usage>(&actions[0]);
    REQUIRE(use);
    CHECK(use->name == "x");
}

TEST_CASE("Match: target and arm usages both appear in sym_seq")
{
    // Target uses outer 't'. Arm uses outer 'a'. Both should appear.
    auto target = std::make_unique<Name_Lookup>(AST_Node::no_range, "t"s);

    auto pat = std::make_unique<Match_Binding>(
        AST_Node::no_range, std::optional<std::string>{"local"}, std::nullopt);
    auto result = std::make_unique<Name_Lookup>(AST_Node::no_range, "a"s);

    std::vector<Match::Arm> arms;
    arms.push_back(arm(std::move(pat), std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    auto actions = m->symbol_sequence() | std::ranges::to<std::vector>();
    std::vector<std::string> names;
    for (const auto& action : actions)
    {
        auto* use = std::get_if<AST_Node::Usage>(&action);
        REQUIRE(use);
        names.push_back(use->name);
    }
    REQUIRE(names.size() == 2);
    // Target comes first, then arm contents.
    CHECK(names[0] == "t");
    CHECK(names[1] == "a");
}

TEST_CASE("Match: empty arms still yields target usages")
{
    auto target = std::make_unique<Name_Lookup>(AST_Node::no_range, "v"s);
    auto m = make_match(std::move(target), {});

    auto actions = m->symbol_sequence() | std::ranges::to<std::vector>();
    REQUIRE(actions.size() == 1);
    auto* use = std::get_if<AST_Node::Usage>(&actions[0]);
    REQUIRE(use);
    CHECK(use->name == "v");
}

TEST_CASE("Match: usages from inside a pattern are yielded through")
{
    // Patterns can contain expressions (e.g. Match_Value with a
    // Name_Lookup, or Match_Map with a computed key) whose
    // symbol_sequence yields Usage actions. Those Usages must escape
    // the Match's symbol_sequence as outer references -- the suppression
    // logic only filters Usages that match a Definition seen earlier in
    // the same arm.
    auto target = mock::Mock_Expression::make();

    // Pattern: Match_Value(outer_var) -- yields use:outer_var
    auto pat = std::make_unique<Match_Value>(
        AST_Node::no_range,
        std::make_unique<Name_Lookup>(AST_Node::no_range, "outer_var"s));

    auto result = mock::Mock_Expression::make();

    std::vector<Match::Arm> arms;
    arms.push_back(arm(std::move(pat), std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    auto actions = m->symbol_sequence() | std::ranges::to<std::vector>();
    REQUIRE(actions.size() == 1);
    auto* use = std::get_if<AST_Node::Usage>(&actions[0]);
    REQUIRE(use);
    CHECK(use->name == "outer_var");
}

TEST_CASE("Match: multiple arms each contribute their own outer usages")
{
    // Each arm has its own per-arm scratch defns set, so independent
    // outer-name usages from different arms must all reach the output.
    auto target = mock::Mock_Expression::make();

    auto pat1 = mock::Mock_Match_Pattern::make();
    auto result1 = std::make_unique<Name_Lookup>(AST_Node::no_range, "outer_a"s);

    auto pat2 = mock::Mock_Match_Pattern::make();
    auto result2 = std::make_unique<Name_Lookup>(AST_Node::no_range, "outer_b"s);

    auto pat3 = mock::Mock_Match_Pattern::make();
    auto result3 = std::make_unique<Name_Lookup>(AST_Node::no_range, "outer_c"s);

    std::vector<Match::Arm> arms;
    arms.push_back(arm(std::move(pat1), std::move(result1)));
    arms.push_back(arm(std::move(pat2), std::move(result2)));
    arms.push_back(arm(std::move(pat3), std::move(result3)));
    auto m = make_match(std::move(target), std::move(arms));

    auto actions = m->symbol_sequence() | std::ranges::to<std::vector>();
    REQUIRE(actions.size() == 3);

    std::vector<std::string> names;
    for (const auto& action : actions)
    {
        auto* use = std::get_if<AST_Node::Usage>(&action);
        REQUIRE(use);
        names.push_back(use->name);
    }
    CHECK(names[0] == "outer_a");
    CHECK(names[1] == "outer_b");
    CHECK(names[2] == "outer_c");
}

TEST_CASE("Match: discard binding produces no Definition and no spurious suppression")
{
    // A Match_Binding with nullopt name yields NO actions. A subsequent
    // result usage of any name should pass through, since there's nothing
    // to suppress against. This guards against a hypothetical bug where
    // a discard pattern accidentally inserted an empty-name or sentinel
    // entry into defns and started suppressing things.
    auto target = mock::Mock_Expression::make();

    auto pat = std::make_unique<Match_Binding>(
        AST_Node::no_range, std::nullopt, std::nullopt);

    auto result = std::make_unique<Name_Lookup>(AST_Node::no_range, "x"s);

    std::vector<Match::Arm> arms;
    arms.push_back(arm(std::move(pat), std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    auto actions = m->symbol_sequence() | std::ranges::to<std::vector>();
    REQUIRE(actions.size() == 1);
    auto* use = std::get_if<AST_Node::Usage>(&actions[0]);
    REQUIRE(use);
    CHECK(use->name == "x");
}

// =============================================================================
// children()
// =============================================================================

TEST_CASE("Match: children begin with the target labeled 'Match_Target'")
{
    auto target = mock::Mock_Expression::make();
    auto* target_raw = target.get();

    auto m = make_match(std::move(target), {});

    auto kids = m->children() | std::ranges::to<std::vector>();
    REQUIRE(kids.size() == 1);
    CHECK(kids[0].node == target_raw);
    CHECK(kids[0].label == "Match_Target");
}

TEST_CASE("Match: single arm without a guard produces Pattern/Result children")
{
    auto target = mock::Mock_Expression::make();
    auto pat = mock::Mock_Match_Pattern::make();
    auto result = mock::Mock_Expression::make();
    auto* target_raw = target.get();
    auto* pat_raw = pat.get();
    auto* result_raw = result.get();

    std::vector<Match::Arm> arms;
    arms.push_back(arm(std::move(pat), std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    auto kids = m->children() | std::ranges::to<std::vector>();
    REQUIRE(kids.size() == 3);

    CHECK(kids[0].node == target_raw);
    CHECK(kids[0].label == "Match_Target");
    CHECK(kids[1].node == pat_raw);
    CHECK(kids[1].label == "Pattern 1");
    CHECK(kids[2].node == result_raw);
    CHECK(kids[2].label == "Result 1");
}

TEST_CASE("Match: single arm with a guard inserts a Guard child")
{
    auto target = mock::Mock_Expression::make();
    auto pat = mock::Mock_Match_Pattern::make();
    auto guard = mock::Mock_Expression::make();
    auto result = mock::Mock_Expression::make();
    auto* target_raw = target.get();
    auto* pat_raw = pat.get();
    auto* guard_raw = guard.get();
    auto* result_raw = result.get();

    std::vector<Match::Arm> arms;
    arms.push_back(guarded_arm(std::move(pat), std::move(guard),
                               std::move(result)));
    auto m = make_match(std::move(target), std::move(arms));

    auto kids = m->children() | std::ranges::to<std::vector>();
    REQUIRE(kids.size() == 4);

    CHECK(kids[0].node == target_raw);
    CHECK(kids[0].label == "Match_Target");
    CHECK(kids[1].node == pat_raw);
    CHECK(kids[1].label == "Pattern 1");
    CHECK(kids[2].node == guard_raw);
    CHECK(kids[2].label == "Guard 1");
    CHECK(kids[3].node == result_raw);
    CHECK(kids[3].label == "Result 1");
}

TEST_CASE("Match: multiple arms produce numbered children in declaration order")
{
    auto target = mock::Mock_Expression::make();
    auto pat1 = mock::Mock_Match_Pattern::make();
    auto result1 = mock::Mock_Expression::make();
    auto pat2 = mock::Mock_Match_Pattern::make();
    auto guard2 = mock::Mock_Expression::make();
    auto result2 = mock::Mock_Expression::make();
    auto pat3 = mock::Mock_Match_Pattern::make();
    auto result3 = mock::Mock_Expression::make();

    auto* target_raw = target.get();
    auto* pat1_raw = pat1.get();
    auto* result1_raw = result1.get();
    auto* pat2_raw = pat2.get();
    auto* guard2_raw = guard2.get();
    auto* result2_raw = result2.get();
    auto* pat3_raw = pat3.get();
    auto* result3_raw = result3.get();

    std::vector<Match::Arm> arms;
    arms.push_back(arm(std::move(pat1), std::move(result1)));
    arms.push_back(
        guarded_arm(std::move(pat2), std::move(guard2), std::move(result2)));
    arms.push_back(arm(std::move(pat3), std::move(result3)));
    auto m = make_match(std::move(target), std::move(arms));

    auto kids = m->children() | std::ranges::to<std::vector>();
    // 1 target + (1 arm with 2 children) + (1 arm with 3 children) +
    // (1 arm with 2 children) = 1 + 2 + 3 + 2 = 8
    REQUIRE(kids.size() == 8);

    CHECK(kids[0].node == target_raw);
    CHECK(kids[0].label == "Match_Target");

    CHECK(kids[1].node == pat1_raw);
    CHECK(kids[1].label == "Pattern 1");
    CHECK(kids[2].node == result1_raw);
    CHECK(kids[2].label == "Result 1");

    CHECK(kids[3].node == pat2_raw);
    CHECK(kids[3].label == "Pattern 2");
    CHECK(kids[4].node == guard2_raw);
    CHECK(kids[4].label == "Guard 2");
    CHECK(kids[5].node == result2_raw);
    CHECK(kids[5].label == "Result 2");

    CHECK(kids[6].node == pat3_raw);
    CHECK(kids[6].label == "Pattern 3");
    CHECK(kids[7].node == result3_raw);
    CHECK(kids[7].label == "Result 3");
}

// =============================================================================
// node_label and source_range
// =============================================================================

TEST_CASE("Match: node_label")
{
    auto target = mock::Mock_Expression::make();
    auto m = make_match(std::move(target), {});
    CHECK(m->node_label() == "Match");
}

TEST_CASE("Match: source_range round-trips through the constructor")
{
    AST_Node::Source_Range range{
        .begin = {.line = 10, .column = 5},
        .end = {.line = 12, .column = 20},
    };

    auto target = mock::Mock_Expression::make();
    auto m = std::make_unique<Match>(range, std::move(target),
                                     std::vector<Match::Arm>{});

    const auto out = m->source_range();
    CHECK(out.begin.line == 10);
    CHECK(out.begin.column == 5);
    CHECK(out.end.line == 12);
    CHECK(out.end.column == 20);
}
