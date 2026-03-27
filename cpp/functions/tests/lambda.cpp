#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>
#include <catch2/trompeloeil.hpp>

#include <frost/ast/function-call.hpp>
#include <frost/ast/lambda.hpp>
#include <frost/closure.hpp>
#include <frost/mock/mock-expression.hpp>
#include <frost/symbol-table.hpp>

#include <memory>
#include <set>

using namespace frst;
using namespace frst::ast;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;
using trompeloeil::_;

namespace
{
struct Seq_Mock_Expression final : mock::Mock_Expression
{
    explicit Seq_Mock_Expression(std::vector<AST_Node::Symbol_Action> seq)
        : seq_{std::move(seq)}
    {
    }

    std::generator<AST_Node::Symbol_Action> symbol_sequence() const final
    {
        for (const auto& action : seq_)
            co_yield action;
    }

  private:
    std::vector<AST_Node::Symbol_Action> seq_;
};

template <typename T, typename... Args>
std::unique_ptr<T> node(Args&&... args)
{
    return std::make_unique<T>(std::forward<Args>(args)...);
}

std::set<std::string> capture_names(const Closure& closure)
{
    std::set<std::string> names;
    for (const auto& entry : closure.debug_capture_table().debug_table())
        names.insert(entry.first);
    return names;
}

std::shared_ptr<const Closure> eval_to_closure(const Lambda& node,
                                               const Symbol_Table& env)
{
    Evaluation_Context ctx{.symbols = env};
    auto result = node.evaluate(ctx);
    REQUIRE(result->is<Function>());
    auto fn = result->get<Function>().value();
    auto closure = std::dynamic_pointer_cast<const Closure>(fn);
    REQUIRE(closure);
    return closure;
}

std::shared_ptr<const Closure> value_to_closure(const Value_Ptr& value)
{
    REQUIRE(value->is<Function>());
    auto fn = value->get<Function>().value();
    auto closure = std::dynamic_pointer_cast<const Closure>(fn);
    REQUIRE(closure);
    return closure;
}
} // namespace

TEST_CASE("Lambda")
{
    // AI-generated test additions by Codex (GPT-5).
    // Signed: Codex (GPT-5).

    SECTION("Evaluating a lambda constructs a function without evaluating body")
    {
        // Frost:
        // def f = fn () -> { side_effect() }
        Symbol_Table env;
        Evaluation_Context ctx{.symbols = env};

        auto expr = std::make_unique<mock::Mock_Expression>();
        auto* expr_ptr = expr.get();
        FORBID_CALL(*expr_ptr, do_evaluate(_));

        std::vector<Statement::Ptr> body;
        body.push_back(std::move(expr));

        Lambda node{AST_Node::no_range, {}, std::move(body)};

        auto result = node.evaluate(ctx);
        CHECK(result->is<Function>());
    }

    SECTION("Symbol sequence yields free usages")
    {
        // Frost:
        // def f = fn (p) -> { x ; def y = 1 ; null }
        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>(AST_Node::no_range, "x"));
        body.push_back(node<Define>(
            AST_Node::no_range, "y",
            node<Literal>(AST_Node::no_range, Value::create(1_f))));
        body.push_back(node<Literal>(AST_Node::no_range, Value::null()));

        Lambda node{AST_Node::no_range, {"p"}, std::move(body)};

        std::vector<AST_Node::Symbol_Action> actions;
        for (const auto& action : node.symbol_sequence())
            actions.push_back(action);

        REQUIRE(actions.size() == 1);
        CHECK(std::holds_alternative<AST_Node::Usage>(actions[0]));
        CHECK(std::get<AST_Node::Usage>(actions[0]).name == "x");
    }

    SECTION("Non-expression-final lambda body is rejected")
    {
        // Frost:
        // def f = fn () -> { def x = 1 }
        std::vector<Statement::Ptr> body;
        body.push_back(node<Define>(
            AST_Node::no_range, "x",
            node<Literal>(AST_Node::no_range, Value::create(1_f))));

        CHECK_THROWS_WITH((Lambda{AST_Node::no_range, {}, std::move(body)}),
                          "A lambda must end in an expression");
    }

    SECTION("Non-expression-final validation runs before shadowing checks")
    {
        // Frost:
        // def f = fn (x) -> { def x = 1 }
        std::vector<Statement::Ptr> body;
        body.push_back(node<Define>(
            AST_Node::no_range, "x",
            node<Literal>(AST_Node::no_range, Value::create(1_f))));

        CHECK_THROWS_WITH((Lambda{AST_Node::no_range, {"x"}, std::move(body)}),
                          "A lambda must end in an expression");
    }

    SECTION("Symbol sequence includes usages in the return expression")
    {
        // Frost:
        // def f = fn () -> { null ; x }
        std::vector<Statement::Ptr> body;
        body.push_back(node<Literal>(AST_Node::no_range, Value::null()));
        body.push_back(node<Name_Lookup>(AST_Node::no_range, "x"));

        Lambda node{AST_Node::no_range, {}, std::move(body)};

        std::vector<AST_Node::Symbol_Action> actions;
        for (const auto& action : node.symbol_sequence())
            actions.push_back(action);

        REQUIRE(actions.size() == 1);
        CHECK(std::holds_alternative<AST_Node::Usage>(actions[0]));
        CHECK(std::get<AST_Node::Usage>(actions[0]).name == "x");
    }

    SECTION("Symbol sequence treats variadic parameter as defined")
    {
        // Frost:
        // def f = fn (...rest) -> { rest }
        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>(AST_Node::no_range, "rest"));

        Lambda node{AST_Node::no_range, {}, std::move(body), "rest"};

        std::vector<AST_Node::Symbol_Action> actions;
        for (const auto& action : node.symbol_sequence())
            actions.push_back(action);

        CHECK(actions.empty());
    }

    SECTION("Empty body with parameters is rejected")
    {
        // Frost:
        // def f = fn (p, q) -> { }
        std::vector<Statement::Ptr> body;

        CHECK_THROWS_WITH(
            (Lambda{AST_Node::no_range, {"p", "q"}, std::move(body)}),
            ContainsSubstring("empty body"));
    }

    SECTION("Empty body with variadic parameter is rejected")
    {
        // Frost:
        // def f = fn (...rest) -> { }
        std::vector<Statement::Ptr> body;

        CHECK_THROWS_WITH(
            (Lambda{AST_Node::no_range, {}, std::move(body), "rest"}),
            ContainsSubstring("empty body"));
    }

    SECTION("Duplicate parameters are rejected at construction")
    {
        // Frost:
        // def f = fn (x, x) -> { }
        std::vector<Statement::Ptr> body;

        CHECK_THROWS_WITH(
            (Lambda{AST_Node::no_range, {"x", "x"}, std::move(body)}),
            "Closure has duplicate parameters");
    }

    SECTION("Variadic parameter cannot duplicate a fixed parameter")
    {
        // Frost:
        // def f = fn (x, ...x) -> { }
        std::vector<Statement::Ptr> body;

        CHECK_THROWS_WITH(
            (Lambda{AST_Node::no_range, {"x"}, std::move(body), "x"}),
            "Closure has duplicate parameters");
    }

    SECTION("Local definition cannot shadow a parameter")
    {
        // Frost:
        // def f = fn (x) -> { def x = 2 ; null }
        std::vector<Statement::Ptr> body;
        body.push_back(node<Define>(
            AST_Node::no_range, "x",
            node<Literal>(AST_Node::no_range, Value::create(2_f))));
        body.push_back(node<Literal>(AST_Node::no_range, Value::null()));

        CHECK_THROWS_WITH((Lambda{AST_Node::no_range, {"x"}, std::move(body)}),
                          ContainsSubstring("parameter")
                              && ContainsSubstring("x"));
    }

    SECTION("Vararg parameter cannot shadow self_name")
    {
        // fn f(...f) -> null  -- "f" as vararg conflicts with self_name "f"
        std::vector<Statement::Ptr> body;
        body.push_back(node<Literal>(AST_Node::no_range, Value::null()));
        CHECK_THROWS_WITH(
            (Lambda{AST_Node::no_range, {}, std::move(body), "f", "f"}),
            ContainsSubstring("f"));
    }

    SECTION("Parameter cannot shadow self_name")
    {
        // fn f(f) -> null  -- "f" as param conflicts with self_name "f"
        std::vector<Statement::Ptr> body;
        body.push_back(node<Literal>(AST_Node::no_range, Value::null()));
        CHECK_THROWS_WITH(
            (Lambda{AST_Node::no_range, {"f"}, std::move(body), {}, "f"}),
            ContainsSubstring("f"));
    }

    SECTION("Local definition cannot shadow self_name")
    {
        // fn f() -> { def f = 1 ; null }  -- local "f" conflicts with self_name
        // "f"
        std::vector<Statement::Ptr> body;
        body.push_back(node<Define>(
            AST_Node::no_range, "f",
            node<Literal>(AST_Node::no_range, Value::create(1_f))));
        body.push_back(node<Literal>(AST_Node::no_range, Value::null()));
        CHECK_THROWS_WITH(
            (Lambda{AST_Node::no_range, {}, std::move(body), {}, "f"}),
            ContainsSubstring("f"));
    }

    SECTION("Symbol sequence treats self_name as defined")
    {
        // fn f(x) -> f(x)  -- "f" is self_name, not a free variable
        std::vector<Expression::Ptr> call_args;
        call_args.push_back(node<Name_Lookup>(AST_Node::no_range, "x"));
        std::vector<Statement::Ptr> body;
        body.push_back(node<Function_Call>(
            AST_Node::no_range, node<Name_Lookup>(AST_Node::no_range, "f"),
            std::move(call_args)));

        Lambda lambda{AST_Node::no_range, {"x"}, std::move(body), {}, "f"};

        std::vector<AST_Node::Symbol_Action> actions;
        for (const auto& action : lambda.symbol_sequence())
            actions.push_back(action);

        CHECK(actions.empty());
    }

    SECTION("Self_name is not captured from environment")
    {
        // def f = 99  (in outer env)
        // fn f(x) -> f(x)  -- "f" is self_name; env's "f" must not be captured
        Symbol_Table env;
        env.define("f", Value::create(99_f));

        std::vector<Expression::Ptr> call_args;
        call_args.push_back(node<Name_Lookup>(AST_Node::no_range, "x"));
        std::vector<Statement::Ptr> body;
        body.push_back(node<Function_Call>(
            AST_Node::no_range, node<Name_Lookup>(AST_Node::no_range, "f"),
            std::move(call_args)));

        Lambda lambda{AST_Node::no_range, {"x"}, std::move(body), {}, "f"};
        auto closure = eval_to_closure(lambda, env);

        // "f" is the self-name -- resolved at call time via shared_from_this,
        // so it should NOT appear in captures
        CHECK_FALSE(closure->debug_capture_table().has("f"));
    }

    SECTION("Named lambda self_name enables recursion")
    {
        // fn fact(n) -> if n <= 1: 1 else: n * fact(n - 1)
        Symbol_Table env;

        std::vector<Expression::Ptr> rec_args;
        rec_args.push_back(node<Binop>(
            AST_Node::no_range, node<Name_Lookup>(AST_Node::no_range, "n"),
            Binary_Op::MINUS,
            node<Literal>(AST_Node::no_range, Value::create(1_f))));
        auto rec_call = node<Function_Call>(
            AST_Node::no_range, node<Name_Lookup>(AST_Node::no_range, "fact"),
            std::move(rec_args));
        auto rec_expr = node<Binop>(AST_Node::no_range,
                                    node<Name_Lookup>(AST_Node::no_range, "n"),
                                    Binary_Op::MULTIPLY, std::move(rec_call));

        std::vector<Statement::Ptr> body;
        body.push_back(node<If>(
            AST_Node::no_range,
            node<Binop>(AST_Node::no_range,
                        node<Name_Lookup>(AST_Node::no_range, "n"),
                        Binary_Op::LE,
                        node<Literal>(AST_Node::no_range, Value::create(1_f))),
            node<Literal>(AST_Node::no_range, Value::create(1_f)),
            std::optional<Expression::Ptr>{std::move(rec_expr)}));

        Lambda lambda{AST_Node::no_range, {"n"}, std::move(body), {}, "fact"};
        auto closure = eval_to_closure(lambda, env);

        auto result = closure->call({Value::create(5_f)});
        REQUIRE(result->is<Int>());
        CHECK(result->raw_get<Int>() == 120_f);
    }

    SECTION("Inner lambda captures self-reference as strong reference")
    {
        // fn f(n) -> fn() -> if n <= 0: 0 else: f(n - 1)()
        //
        // When f(1) is called, the inner lambda is created. "f" resolves to
        // the self-reference defined on f's call stack via shared_from_this.
        // The inner lambda captures this as a normal strong reference.
        //
        // After dropping the outer strong reference to f's closure, only the
        // inner lambda's capture keeps it alive.
        Symbol_Table env;

        // f(n - 1)
        std::vector<Expression::Ptr> rec_args;
        rec_args.push_back(node<Binop>(
            AST_Node::no_range, node<Name_Lookup>(AST_Node::no_range, "n"),
            Binary_Op::MINUS,
            node<Literal>(AST_Node::no_range, Value::create(1_f))));
        auto f_of_nm1 = node<Function_Call>(
            AST_Node::no_range, node<Name_Lookup>(AST_Node::no_range, "f"),
            std::move(rec_args));
        // f(n - 1)()
        auto f_of_nm1_called =
            node<Function_Call>(AST_Node::no_range, std::move(f_of_nm1),
                                std::vector<Expression::Ptr>{});

        // fn() -> if n <= 0: 0 else: f(n - 1)()
        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(node<If>(
            AST_Node::no_range,
            node<Binop>(AST_Node::no_range,
                        node<Name_Lookup>(AST_Node::no_range, "n"),
                        Binary_Op::LE,
                        node<Literal>(AST_Node::no_range, Value::create(0_f))),
            node<Literal>(AST_Node::no_range, Value::create(0_f)),
            std::optional<Expression::Ptr>{std::move(f_of_nm1_called)}));
        auto inner_lambda =
            node<Lambda>(AST_Node::no_range, std::vector<std::string>{},
                         std::move(inner_body));

        // fn f(n) -> fn() -> ...
        std::vector<Statement::Ptr> outer_body;
        outer_body.push_back(std::move(inner_lambda));
        Lambda outer{
            AST_Node::no_range, {"n"}, std::move(outer_body), {}, "f"};

        Value_Ptr inner_val;
        {
            auto outer_closure = eval_to_closure(outer, env);
            inner_val = outer_closure->call({Value::create(1_f)});
            // outer_closure dropped here; f's Closure survives only if
            // inner_val holds a strong reference to it.
        }

        // inner() -> if 1 <= 0: 0 else: f(0)()
        // f(0)() -> if 0 <= 0: 0 -> 0
        REQUIRE(inner_val->is<Function>());
        auto result = inner_val->get<Function>().value()->call({});
        REQUIRE(result->is<Int>());
        CHECK(result->raw_get<Int>() == 0_f);
    }

    SECTION("Named lambda closes over outer variables")
    {
        // def base = 42
        // fn f(n) -> if n <= 0: base else: f(n - 1)
        Symbol_Table env;
        auto base_val = Value::create(42_f);
        env.define("base", base_val);

        std::vector<Expression::Ptr> rec_args;
        rec_args.push_back(node<Binop>(
            AST_Node::no_range, node<Name_Lookup>(AST_Node::no_range, "n"),
            Binary_Op::MINUS,
            node<Literal>(AST_Node::no_range, Value::create(1_f))));
        auto rec_call = node<Function_Call>(
            AST_Node::no_range, node<Name_Lookup>(AST_Node::no_range, "f"),
            std::move(rec_args));

        std::vector<Statement::Ptr> body;
        body.push_back(node<If>(
            AST_Node::no_range,
            node<Binop>(AST_Node::no_range,
                        node<Name_Lookup>(AST_Node::no_range, "n"),
                        Binary_Op::LE,
                        node<Literal>(AST_Node::no_range, Value::create(0_f))),
            node<Name_Lookup>(AST_Node::no_range, "base"),
            std::optional<Expression::Ptr>{std::move(rec_call)}));

        Lambda lambda{AST_Node::no_range, {"n"}, std::move(body), {}, "f"};
        auto closure = eval_to_closure(lambda, env);

        // capture set should contain "base" only; "f" is the self-name,
        // resolved at call time via shared_from_this
        CHECK(capture_names(*closure) == std::set<std::string>{"base"});
        CHECK(closure->debug_capture_table().lookup("base") == base_val);

        auto result = closure->call({Value::create(3_f)});
        REQUIRE(result->is<Int>());
        CHECK(result->raw_get<Int>() == 42_f);
    }

    SECTION("Local definition cannot shadow a variadic parameter")
    {
        // Frost:
        // def f = fn (...rest) -> { def rest = 2 ; null }
        std::vector<Statement::Ptr> body;
        body.push_back(node<Define>(
            AST_Node::no_range, "rest",
            node<Literal>(AST_Node::no_range, Value::create(2_f))));
        body.push_back(node<Literal>(AST_Node::no_range, Value::null()));

        CHECK_THROWS_WITH(
            (Lambda{AST_Node::no_range, {}, std::move(body), "rest"}),
            ContainsSubstring("parameter") && ContainsSubstring("rest"));
    }

    SECTION("Captures only free variables")
    {
        // Frost:
        // def x = 10
        // def y = 20
        // def f = fn () -> { x + y }
        Symbol_Table env;
        auto x_val = Value::create(10_f);
        auto y_val = Value::create(20_f);
        env.define("x", x_val);
        env.define("y", y_val);

        std::vector<Statement::Ptr> body;
        body.push_back(node<Binop>(
            AST_Node::no_range, node<Name_Lookup>(AST_Node::no_range, "x"),
            Binary_Op::PLUS, node<Name_Lookup>(AST_Node::no_range, "y")));

        Lambda node{AST_Node::no_range, {}, std::move(body)};
        auto closure = eval_to_closure(node, env);

        CHECK(capture_names(*closure) == std::set<std::string>{"x", "y"});
        CHECK(closure->debug_capture_table().lookup("x") == x_val);
        CHECK(closure->debug_capture_table().lookup("y") == y_val);
    }

    SECTION("Captures through a failover table")
    {
        // Frost:
        // def x = 42
        // def f = fn () -> { x }
        Symbol_Table outer;
        auto x_val = Value::create(42_f);
        outer.define("x", x_val);

        Symbol_Table env{&outer};

        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>(AST_Node::no_range, "x"));

        Lambda node{AST_Node::no_range, {}, std::move(body)};
        auto closure = eval_to_closure(node, env);

        CHECK(capture_names(*closure) == std::set<std::string>{"x"});
        CHECK(closure->debug_capture_table().lookup("x") == x_val);
    }

    SECTION("Parameters are not captured")
    {
        // Frost:
        // def p = 1
        // def x = 2
        // def f = fn (p) -> { p + x }
        Symbol_Table env;
        auto p_val = Value::create(1_f);
        auto x_val = Value::create(2_f);
        env.define("p", p_val);
        env.define("x", x_val);

        std::vector<Statement::Ptr> body;
        body.push_back(node<Binop>(
            AST_Node::no_range, node<Name_Lookup>(AST_Node::no_range, "p"),
            Binary_Op::PLUS, node<Name_Lookup>(AST_Node::no_range, "x")));

        Lambda node{AST_Node::no_range, {"p"}, std::move(body)};
        auto closure = eval_to_closure(node, env);

        CHECK(capture_names(*closure) == std::set<std::string>{"x"});
        CHECK(closure->debug_capture_table().lookup("x") == x_val);
        CHECK_FALSE(closure->debug_capture_table().has("p"));
    }

    SECTION("Variadic parameter is not captured from the environment")
    {
        // Frost:
        // def rest = 99
        // def f = fn (...rest) -> { rest }
        Symbol_Table env;
        env.define("rest", Value::create(99_f));

        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>(AST_Node::no_range, "rest"));

        Lambda node{AST_Node::no_range, {}, std::move(body), "rest"};
        auto closure = eval_to_closure(node, env);

        CHECK(capture_names(*closure).empty());

        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto result = closure->call({a, b});
        REQUIRE(result->is<Array>());
        auto arr = result->get<Array>();
        REQUIRE(arr->size() == 2);
        CHECK(arr->at(0) == a);
        CHECK(arr->at(1) == b);
    }

    SECTION("Inner variadic parameter does not cause outer capture")
    {
        // Frost:
        // def rest = 99
        // def outer = fn () -> { fn (...rest) -> { rest } }
        Symbol_Table env;
        env.define("rest", Value::create(99_f));

        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(node<Name_Lookup>(AST_Node::no_range, "rest"));

        std::vector<Statement::Ptr> outer_body;
        outer_body.push_back(node<Lambda>(AST_Node::no_range,
                                          std::vector<std::string>{},
                                          std::move(inner_body), "rest"));

        Lambda outer{AST_Node::no_range, {}, std::move(outer_body)};
        auto outer_closure = eval_to_closure(outer, env);

        CHECK(capture_names(*outer_closure).empty());
    }

    SECTION("Named inner lambda self_name not captured by outer")
    {
        // def f = 99  (in outer env)
        // def outer = fn () -> { fn f(x) -> f(x) }
        // outer should NOT capture "f" from env; inner lambda handles it itself
        Symbol_Table env;
        env.define("f", Value::create(99_f));

        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(node<Name_Lookup>(AST_Node::no_range, "f"));

        std::vector<Statement::Ptr> outer_body;
        outer_body.push_back(
            node<Lambda>(AST_Node::no_range, std::vector<std::string>{"x"},
                         std::move(inner_body), std::optional<std::string>{},
                         std::optional<std::string>{"f"}));

        Lambda outer{AST_Node::no_range, {}, std::move(outer_body)};
        auto outer_closure = eval_to_closure(outer, env);

        CHECK(capture_names(*outer_closure).empty());
    }

    SECTION("Inner parameter shadows outer variadic parameter")
    {
        // Frost:
        // def outer = fn (...rest) -> { fn (rest) -> { rest } }
        Symbol_Table env;

        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(node<Name_Lookup>(AST_Node::no_range, "rest"));

        std::vector<Statement::Ptr> outer_body;
        outer_body.push_back(node<Lambda>(AST_Node::no_range,
                                          std::vector<std::string>{"rest"},
                                          std::move(inner_body)));

        Lambda outer{AST_Node::no_range, {}, std::move(outer_body), "rest"};
        auto outer_closure = eval_to_closure(outer, env);

        CHECK(capture_names(*outer_closure).empty());

        auto inner_val = outer_closure->call({Value::create(1_f)});
        auto inner_closure = value_to_closure(inner_val);
        CHECK(capture_names(*inner_closure).empty());
    }

    SECTION("Inner variadic parameter shadows outer variadic parameter")
    {
        // Frost:
        // def outer = fn (...rest) -> { fn (...rest) -> { rest } }
        Symbol_Table env;

        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(node<Name_Lookup>(AST_Node::no_range, "rest"));

        std::vector<Statement::Ptr> outer_body;
        outer_body.push_back(node<Lambda>(AST_Node::no_range,
                                          std::vector<std::string>{},
                                          std::move(inner_body), "rest"));

        Lambda outer{AST_Node::no_range, {}, std::move(outer_body), "rest"};
        auto outer_closure = eval_to_closure(outer, env);

        CHECK(capture_names(*outer_closure).empty());

        auto inner_val = outer_closure->call({Value::create(1_f)});
        auto inner_closure = value_to_closure(inner_val);
        CHECK(capture_names(*inner_closure).empty());
    }

    SECTION("Inner lambda captures outer vararg array")
    {
        // Frost:
        // def outer = fn (...rest) -> { fn () -> { rest } }
        Symbol_Table env;

        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(node<Name_Lookup>(AST_Node::no_range, "rest"));

        std::vector<Statement::Ptr> outer_body;
        outer_body.push_back(node<Lambda>(AST_Node::no_range,
                                          std::vector<std::string>{},
                                          std::move(inner_body)));

        Lambda outer{AST_Node::no_range, {}, std::move(outer_body), "rest"};
        auto outer_closure = eval_to_closure(outer, env);

        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto inner_val = outer_closure->call({a, b});
        auto inner_closure = value_to_closure(inner_val);

        auto out = inner_closure->call({});
        REQUIRE(out->is<Array>());
        auto arr = out->get<Array>();
        REQUIRE(arr->size() == 2);
        CHECK(arr->at(0) == a);
        CHECK(arr->at(1) == b);
    }

    SECTION("Use before define captures from environment")
    {
        // Frost:
        // def x = 5;
        // def f = fn () -> { x + 1 ; def x = 2 ; null }
        Symbol_Table env;
        auto x_val = Value::create(5_f);
        env.define("x", x_val);

        std::vector<Statement::Ptr> body;
        body.push_back(node<Binop>(
            AST_Node::no_range, node<Name_Lookup>(AST_Node::no_range, "x"),
            Binary_Op::PLUS,
            node<Literal>(AST_Node::no_range, Value::create(1_f))));
        body.push_back(node<Define>(
            AST_Node::no_range, "x",
            node<Literal>(AST_Node::no_range, Value::create(2_f))));
        body.push_back(node<Literal>(AST_Node::no_range, Value::null()));

        Lambda node{AST_Node::no_range, {}, std::move(body)};
        auto closure = eval_to_closure(node, env);

        CHECK(capture_names(*closure) == std::set<std::string>{"x"});
        CHECK(closure->debug_capture_table().lookup("x") == x_val);
    }

    SECTION("Locals shadowing environment are not captured")
    {
        // Frost:
        // def x = 5
        // def y = 7
        // def f = fn () -> { def x = y ; x }
        Symbol_Table env;
        auto x_val = Value::create(5_f);
        auto y_val = Value::create(7_f);
        env.define("x", x_val);
        env.define("y", y_val);

        std::vector<Statement::Ptr> body;
        body.push_back(
            node<Define>(AST_Node::no_range, "x",
                         node<Name_Lookup>(AST_Node::no_range, "y")));
        body.push_back(node<Name_Lookup>(AST_Node::no_range, "x"));

        Lambda node{AST_Node::no_range, {}, std::move(body)};
        auto closure = eval_to_closure(node, env);

        CHECK(capture_names(*closure) == std::set<std::string>{"y"});
        CHECK(closure->debug_capture_table().lookup("y") == y_val);
        CHECK_FALSE(closure->debug_capture_table().has("x"));
    }

    SECTION("Use before and after local define")
    {
        // Frost:
        // def x = 10
        // def y = 20
        // def f = fn () -> { x + y ; def x = 3 ; x }
        Symbol_Table env;
        auto x_val = Value::create(10_f);
        auto y_val = Value::create(20_f);
        env.define("x", x_val);
        env.define("y", y_val);

        std::vector<Statement::Ptr> body;
        body.push_back(node<Binop>(
            AST_Node::no_range, node<Name_Lookup>(AST_Node::no_range, "x"),
            Binary_Op::PLUS, node<Name_Lookup>(AST_Node::no_range, "y")));
        body.push_back(node<Define>(
            AST_Node::no_range, "x",
            node<Literal>(AST_Node::no_range, Value::create(3_f))));
        body.push_back(node<Name_Lookup>(AST_Node::no_range, "x"));

        Lambda node{AST_Node::no_range, {}, std::move(body)};
        auto closure = eval_to_closure(node, env);

        CHECK(capture_names(*closure) == std::set<std::string>{"x", "y"});
        CHECK(closure->debug_capture_table().lookup("x") == x_val);
        CHECK(closure->debug_capture_table().lookup("y") == y_val);
    }

    SECTION("Definition without usage does not capture")
    {
        // Frost:
        // def x = 5
        // def f = fn () -> { def x = 7 ; null }
        Symbol_Table env;
        auto x_val = Value::create(5_f);
        env.define("x", x_val);

        std::vector<Statement::Ptr> body;
        body.push_back(node<Define>(
            AST_Node::no_range, "x",
            node<Literal>(AST_Node::no_range, Value::create(7_f))));
        body.push_back(node<Literal>(AST_Node::no_range, Value::null()));

        Lambda node{AST_Node::no_range, {}, std::move(body)};
        auto closure = eval_to_closure(node, env);

        CHECK(capture_names(*closure).empty());
        CHECK_FALSE(closure->debug_capture_table().has("x"));
    }

    SECTION("Self-referential define captures prior usage in expression")
    {
        // Frost:
        // def x = 10
        // def f = fn () -> { def x = 1 + x ; null }
        Symbol_Table env;
        auto x_val = Value::create(10_f);
        env.define("x", x_val);

        std::vector<Statement::Ptr> body;
        body.push_back(node<Define>(
            AST_Node::no_range, "x",
            node<Binop>(AST_Node::no_range,
                        node<Literal>(AST_Node::no_range, Value::create(1_f)),
                        Binary_Op::PLUS,
                        node<Name_Lookup>(AST_Node::no_range, "x"))));
        body.push_back(node<Literal>(AST_Node::no_range, Value::null()));

        Lambda node{AST_Node::no_range, {}, std::move(body)};
        auto closure = eval_to_closure(node, env);

        CHECK(capture_names(*closure) == std::set<std::string>{"x"});
        CHECK(closure->debug_capture_table().lookup("x") == x_val);
    }

    SECTION("Parameter used only in define RHS is not captured")
    {
        // Frost:
        // def p = 3
        // def f = fn (p) -> { def x = p + 1 ; null }
        Symbol_Table env;
        auto p_val = Value::create(3_f);
        env.define("p", p_val);

        std::vector<Statement::Ptr> body;
        body.push_back(node<Define>(
            AST_Node::no_range, "x",
            node<Binop>(
                AST_Node::no_range,
                node<Name_Lookup>(AST_Node::no_range, "p"), Binary_Op::PLUS,
                node<Literal>(AST_Node::no_range, Value::create(1_f)))));
        body.push_back(node<Literal>(AST_Node::no_range, Value::null()));

        Lambda node{AST_Node::no_range, {"p"}, std::move(body)};
        auto closure = eval_to_closure(node, env);

        CHECK(capture_names(*closure).empty());
        CHECK_FALSE(closure->debug_capture_table().has("p"));
    }

    SECTION("If captures condition and both branches (structural)")
    {
        // Frost:
        // def cond = true
        // def t = 1
        // def f = 0
        // def g = fn () -> { if cond: t else: f }
        Symbol_Table env;
        auto cond_val = Value::create(true);
        auto t_val = Value::create(1_f);
        auto f_val = Value::create(0_f);
        env.define("cond", cond_val);
        env.define("t", t_val);
        env.define("f", f_val);

        std::vector<Statement::Ptr> body;
        body.push_back(node<If>(
            AST_Node::no_range, node<Name_Lookup>(AST_Node::no_range, "cond"),
            node<Name_Lookup>(AST_Node::no_range, "t"),
            std::optional<Expression::Ptr>{
                node<Name_Lookup>(AST_Node::no_range, "f")}));

        Lambda node{AST_Node::no_range, {}, std::move(body)};
        auto closure = eval_to_closure(node, env);

        CHECK(capture_names(*closure)
              == std::set<std::string>{"cond", "f", "t"});
        CHECK(closure->debug_capture_table().lookup("cond") == cond_val);
        CHECK(closure->debug_capture_table().lookup("t") == t_val);
        CHECK(closure->debug_capture_table().lookup("f") == f_val);
    }

    SECTION("Missing capture throws during evaluate")
    {
        // Frost:
        // def f = fn () -> { missing }
        Symbol_Table env;
        Evaluation_Context ctx{.symbols = env};

        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>(AST_Node::no_range, "missing"));

        Lambda node{AST_Node::no_range, {}, std::move(body)};

        CHECK_THROWS_WITH(
            node.evaluate(ctx),
            ContainsSubstring("No definition found for captured symbol"));
    }

    SECTION("Mock expression sequence controls capture set")
    {
        // Frost (symbol-sequence only; not a real AST):
        // use a, def b, use b, use c
        Symbol_Table env;
        auto a_val = Value::create(1_f);
        auto c_val = Value::create(2_f);
        env.define("a", a_val);
        env.define("c", c_val);

        std::vector<AST_Node::Symbol_Action> seq{
            AST_Node::Usage{"a"},
            AST_Node::Definition{"b"},
            AST_Node::Usage{"b"},
            AST_Node::Usage{"c"},
        };

        std::vector<Statement::Ptr> body;
        body.push_back(node<Seq_Mock_Expression>(std::move(seq)));

        Lambda node{AST_Node::no_range, {}, std::move(body)};
        auto closure = eval_to_closure(node, env);

        CHECK(capture_names(*closure) == std::set<std::string>{"a", "c"});
        CHECK(closure->debug_capture_table().lookup("a") == a_val);
        CHECK(closure->debug_capture_table().lookup("c") == c_val);
        CHECK_FALSE(closure->debug_capture_table().has("b"));
    }

    SECTION("Evaluated lambda captures environment and uses parameters")
    {
        // Frost:
        // def x = 2
        // def f = fn (y) -> { x + y }
        Symbol_Table env;
        Evaluation_Context ctx{.symbols = env};
        env.define("x", Value::create(2_f));

        std::vector<Statement::Ptr> body;
        body.push_back(node<Binop>(
            AST_Node::no_range, node<Name_Lookup>(AST_Node::no_range, "x"),
            Binary_Op::PLUS, node<Name_Lookup>(AST_Node::no_range, "y")));

        Lambda node{AST_Node::no_range, {"y"}, std::move(body)};

        auto result = node.evaluate(ctx);
        REQUIRE(result->is<Function>());
        auto fn = result->get<Function>().value();

        auto out = fn->call({Value::create(4_f)});
        CHECK(out->get<Int>() == 6_f);
    }

    SECTION("Captures are read from the evaluation environment")
    {
        // Frost:
        // def f = fn () -> { x }
        // def f1 = f in env1, def f2 = f in env2
        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>(AST_Node::no_range, "x"));

        Lambda node{AST_Node::no_range, {}, std::move(body)};

        Symbol_Table env1;
        auto v1 = Value::create(1_f);
        env1.define("x", v1);

        Symbol_Table env2;
        auto v2 = Value::create(2_f);
        env2.define("x", v2);

        auto closure1 = eval_to_closure(node, env1);
        auto closure2 = eval_to_closure(node, env2);

        CHECK(closure1->debug_capture_table().lookup("x") == v1);
        CHECK(closure2->debug_capture_table().lookup("x") == v2);
    }

    SECTION("Evaluating the same lambda twice constructs distinct closures")
    {
        // Frost:
        // def x = 1
        // def f = fn () -> { x }
        // f evaluated twice yields distinct closures
        Symbol_Table env;
        env.define("x", Value::create(1_f));

        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>(AST_Node::no_range, "x"));

        Lambda node{AST_Node::no_range, {}, std::move(body)};

        auto closure1 = eval_to_closure(node, env);
        auto closure2 = eval_to_closure(node, env);

        CHECK(closure1 != closure2);
    }

    // AI-generated nested-lambda tests by Codex (GPT-5).
    SECTION("Nested lambda captures outer locals and globals")
    {
        // Frost:
        // def x = 1
        // def outer = fn () -> {
        //     def y = 2
        //     fn () -> { x + y }
        // }
        Symbol_Table env;
        auto x_val = Value::create(1_f);
        auto y_val = Value::create(2_f);
        env.define("x", x_val);

        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(node<Binop>(
            AST_Node::no_range, node<Name_Lookup>(AST_Node::no_range, "x"),
            Binary_Op::PLUS, node<Name_Lookup>(AST_Node::no_range, "y")));

        std::vector<Statement::Ptr> outer_body;
        outer_body.push_back(
            node<Define>(AST_Node::no_range, "y",
                         node<Literal>(AST_Node::no_range, y_val)));
        outer_body.push_back(node<Lambda>(AST_Node::no_range,
                                          std::vector<std::string>{},
                                          std::move(inner_body)));

        Lambda outer{AST_Node::no_range, {}, std::move(outer_body)};

        auto outer_closure = eval_to_closure(outer, env);
        CHECK(capture_names(*outer_closure) == std::set<std::string>{"x"});
        CHECK(outer_closure->debug_capture_table().lookup("x") == x_val);

        auto inner_val = outer_closure->call({});
        auto inner_closure = value_to_closure(inner_val);

        CHECK(capture_names(*inner_closure) == std::set<std::string>{"x", "y"});
        CHECK(inner_closure->debug_capture_table().lookup("x") == x_val);
        CHECK(inner_closure->debug_capture_table().lookup("y") == y_val);

        auto out = inner_closure->call({});
        CHECK(out->get<Int>() == 3_f);
    }

    SECTION("Nested lambda captures outer parameter per call")
    {
        // Frost:
        // def outer = fn (p) -> { fn () -> { p } }
        // def f1 = outer(10)
        // def f2 = outer(20)
        Symbol_Table env;

        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(node<Name_Lookup>(AST_Node::no_range, "p"));

        std::vector<Statement::Ptr> outer_body;
        outer_body.push_back(node<Lambda>(AST_Node::no_range,
                                          std::vector<std::string>{},
                                          std::move(inner_body)));

        Lambda outer{AST_Node::no_range, {"p"}, std::move(outer_body)};
        auto outer_closure = eval_to_closure(outer, env);

        auto p1 = Value::create(10_f);
        auto p2 = Value::create(20_f);

        auto inner_val1 = outer_closure->call({p1});
        auto inner_val2 = outer_closure->call({p2});

        auto inner1 = value_to_closure(inner_val1);
        auto inner2 = value_to_closure(inner_val2);

        CHECK(capture_names(*inner1) == std::set<std::string>{"p"});
        CHECK(capture_names(*inner2) == std::set<std::string>{"p"});
        CHECK(inner1->debug_capture_table().lookup("p") == p1);
        CHECK(inner2->debug_capture_table().lookup("p") == p2);

        CHECK(inner1->call({}) == p1);
        CHECK(inner2->call({}) == p2);
    }

    SECTION("Nested lambda uses preceding global definition")
    {
        // Frost:
        // def x = 1
        // def outer = fn () -> {
        //     def inner = fn () -> { x }
        //     def x = 2
        //     inner
        // }
        Symbol_Table env;
        auto x_val = Value::create(1_f);
        env.define("x", x_val);

        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(node<Name_Lookup>(AST_Node::no_range, "x"));

        std::vector<Statement::Ptr> outer_body;
        outer_body.push_back(node<Define>(
            AST_Node::no_range, "inner",
            node<Lambda>(AST_Node::no_range, std::vector<std::string>{},
                         std::move(inner_body))));
        outer_body.push_back(node<Define>(
            AST_Node::no_range, "x",
            node<Literal>(AST_Node::no_range, Value::create(2_f))));
        outer_body.push_back(node<Name_Lookup>(AST_Node::no_range, "inner"));

        Lambda outer{AST_Node::no_range, {}, std::move(outer_body)};
        auto outer_closure = eval_to_closure(outer, env);
        CHECK(capture_names(*outer_closure) == std::set<std::string>{"x"});

        auto inner_val = outer_closure->call({});
        auto inner_closure = value_to_closure(inner_val);

        CHECK(inner_closure->debug_capture_table().lookup("x") == x_val);
        CHECK(inner_closure->call({}) == x_val);
    }

    SECTION("Nested lambda use-before-define is an error")
    {
        // Frost:
        // def outer = fn () -> {
        //     fn () -> { z }
        //     def z = 42
        //     null
        // }
        Symbol_Table env;
        Evaluation_Context ctx{.symbols = env};

        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(node<Name_Lookup>(AST_Node::no_range, "z"));

        std::vector<Statement::Ptr> outer_body;
        outer_body.push_back(node<Lambda>(AST_Node::no_range,
                                          std::vector<std::string>{},
                                          std::move(inner_body)));
        outer_body.push_back(node<Define>(
            AST_Node::no_range, "z",
            node<Literal>(AST_Node::no_range, Value::create(42_f))));
        outer_body.push_back(node<Literal>(AST_Node::no_range, Value::null()));

        Lambda outer{AST_Node::no_range, {}, std::move(outer_body)};

        CHECK_THROWS_WITH(
            outer.evaluate(ctx),
            ContainsSubstring("No definition found for captured symbol")
                && ContainsSubstring("z"));
    }

    SECTION("Three-level nesting captures through intermediate lambda")
    {
        // Frost:
        // def x = 1
        // def outer = fn () -> {
        //     def mid = fn () -> { fn () -> { x } }
        //     mid
        // }
        Symbol_Table env;
        auto x_val = Value::create(1_f);
        env.define("x", x_val);

        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(node<Name_Lookup>(AST_Node::no_range, "x"));

        std::vector<Statement::Ptr> mid_body;
        mid_body.push_back(node<Lambda>(AST_Node::no_range,
                                        std::vector<std::string>{},
                                        std::move(inner_body)));

        std::vector<Statement::Ptr> outer_body;
        outer_body.push_back(node<Lambda>(AST_Node::no_range,
                                          std::vector<std::string>{},
                                          std::move(mid_body)));

        Lambda outer{AST_Node::no_range, {}, std::move(outer_body)};
        auto outer_closure = eval_to_closure(outer, env);
        CHECK(capture_names(*outer_closure) == std::set<std::string>{"x"});

        auto mid_val = outer_closure->call({});
        auto mid_closure = value_to_closure(mid_val);
        CHECK(capture_names(*mid_closure) == std::set<std::string>{"x"});

        auto inner_val = mid_closure->call({});
        auto inner_closure = value_to_closure(inner_val);
        CHECK(capture_names(*inner_closure) == std::set<std::string>{"x"});

        auto out = inner_closure->call({});
        CHECK(out == x_val);
    }

    SECTION("Nested lambda captures structured values for map values")
    {
        // Frost:
        // def outer = fn () -> {
        //     def a = [1]
        //     def b = [1]
        //     fn () -> {
        //         def m = { "a": a, "b": b }
        //         [ m["a"], m["b"] ]
        //     }
        // }
        Symbol_Table env;

        auto one = Value::create(1_f);
        auto a_key = Value::create(String{"a"});
        auto b_key = Value::create(String{"b"});

        std::vector<Statement::Ptr> inner_body;
        {
            std::vector<Map_Constructor::KV_Pair> pairs;
            pairs.emplace_back(node<Literal>(AST_Node::no_range, a_key),
                               node<Name_Lookup>(AST_Node::no_range, "a"));
            pairs.emplace_back(node<Literal>(AST_Node::no_range, b_key),
                               node<Name_Lookup>(AST_Node::no_range, "b"));

            inner_body.push_back(node<Define>(
                AST_Node::no_range, "m",
                node<Map_Constructor>(AST_Node::no_range, std::move(pairs))));

            std::vector<Expression::Ptr> elems;
            elems.push_back(
                node<Index>(AST_Node::no_range,
                            node<Name_Lookup>(AST_Node::no_range, "m"),
                            node<Literal>(AST_Node::no_range, a_key)));
            elems.push_back(
                node<Index>(AST_Node::no_range,
                            node<Name_Lookup>(AST_Node::no_range, "m"),
                            node<Literal>(AST_Node::no_range, b_key)));

            inner_body.push_back(
                node<Array_Constructor>(AST_Node::no_range, std::move(elems)));
        }

        std::vector<Statement::Ptr> outer_body;
        {
            std::vector<Expression::Ptr> a_elems;
            a_elems.push_back(node<Literal>(AST_Node::no_range, one));
            std::vector<Expression::Ptr> b_elems;
            b_elems.push_back(node<Literal>(AST_Node::no_range, one));

            outer_body.push_back(
                node<Define>(AST_Node::no_range, "a",
                             node<Array_Constructor>(AST_Node::no_range,
                                                     std::move(a_elems))));
            outer_body.push_back(
                node<Define>(AST_Node::no_range, "b",
                             node<Array_Constructor>(AST_Node::no_range,
                                                     std::move(b_elems))));
            outer_body.push_back(node<Lambda>(AST_Node::no_range,
                                              std::vector<std::string>{},
                                              std::move(inner_body)));
        }

        Lambda outer{AST_Node::no_range, {}, std::move(outer_body)};
        auto outer_closure = eval_to_closure(outer, env);

        auto inner_val = outer_closure->call({});
        auto inner_closure = value_to_closure(inner_val);

        auto out = inner_closure->call({});
        auto arr = out->get<Array>().value();
        REQUIRE(arr.size() == 2);
        REQUIRE(arr[0]->is<Array>());
        REQUIRE(arr[1]->is<Array>());
        CHECK(arr[0]->raw_get<Array>().size() == 1);
        CHECK(arr[1]->raw_get<Array>().size() == 1);
        CHECK(arr[0]->raw_get<Array>().front()->get<Int>().value() == 1_f);
        CHECK(arr[1]->raw_get<Array>().front()->get<Int>().value() == 1_f);
    }

    SECTION("Nested lambda parameter shadows outer name")
    {
        // Frost:
        // def x = 10
        // def outer = fn () -> { fn (x) -> { x } }
        Symbol_Table env;
        auto x_val = Value::create(10_f);
        env.define("x", x_val);

        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(node<Name_Lookup>(AST_Node::no_range, "x"));

        std::vector<Statement::Ptr> outer_body;
        outer_body.push_back(node<Lambda>(AST_Node::no_range,
                                          std::vector<std::string>{"x"},
                                          std::move(inner_body)));

        Lambda outer{AST_Node::no_range, {}, std::move(outer_body)};
        auto outer_closure = eval_to_closure(outer, env);
        CHECK(capture_names(*outer_closure).empty());

        auto inner_val = outer_closure->call({});
        auto inner_closure = value_to_closure(inner_val);
        CHECK(capture_names(*inner_closure).empty());

        auto arg = Value::create(20_f);
        CHECK(inner_closure->call({arg}) == arg);
    }

    SECTION("Nested lambda captures outer parameter but not inner parameter")
    {
        // Frost:
        // def outer = fn (p) -> { fn (q) -> { p + q } }
        Symbol_Table env;

        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(node<Binop>(
            AST_Node::no_range, node<Name_Lookup>(AST_Node::no_range, "p"),
            Binary_Op::PLUS, node<Name_Lookup>(AST_Node::no_range, "q")));

        std::vector<Statement::Ptr> outer_body;
        outer_body.push_back(node<Lambda>(AST_Node::no_range,
                                          std::vector<std::string>{"q"},
                                          std::move(inner_body)));

        Lambda outer{AST_Node::no_range, {"p"}, std::move(outer_body)};
        auto outer_closure = eval_to_closure(outer, env);
        CHECK(capture_names(*outer_closure).empty());

        auto p_val = Value::create(3_f);
        auto inner_val = outer_closure->call({p_val});
        auto inner_closure = value_to_closure(inner_val);

        CHECK(capture_names(*inner_closure) == std::set<std::string>{"p"});
        CHECK(inner_closure->debug_capture_table().lookup("p") == p_val);

        auto out = inner_closure->call({Value::create(4_f)});
        CHECK(out->get<Int>() == 7_f);
    }

    SECTION("Nested lambda captures global, local, and parameter")
    {
        // Frost:
        // def x = 1
        // def outer = fn (p) -> {
        //     def y = 2
        //     fn (q) -> { x + y + p + q }
        // }
        Symbol_Table env;
        auto x_val = Value::create(1_f);
        env.define("x", x_val);

        auto y_val = Value::create(2_f);

        std::vector<Statement::Ptr> inner_body;
        {
            auto sum_xy = node<Binop>(
                AST_Node::no_range,
                node<Name_Lookup>(AST_Node::no_range, "x"), Binary_Op::PLUS,
                node<Name_Lookup>(AST_Node::no_range, "y"));
            auto sum_xyp = node<Binop>(
                AST_Node::no_range, std::move(sum_xy), Binary_Op::PLUS,
                node<Name_Lookup>(AST_Node::no_range, "p"));
            inner_body.push_back(node<Binop>(
                AST_Node::no_range, std::move(sum_xyp), Binary_Op::PLUS,
                node<Name_Lookup>(AST_Node::no_range, "q")));
        }

        std::vector<Statement::Ptr> outer_body;
        outer_body.push_back(
            node<Define>(AST_Node::no_range, "y",
                         node<Literal>(AST_Node::no_range, y_val)));
        outer_body.push_back(node<Lambda>(AST_Node::no_range,
                                          std::vector<std::string>{"q"},
                                          std::move(inner_body)));

        Lambda outer{AST_Node::no_range, {"p"}, std::move(outer_body)};
        auto outer_closure = eval_to_closure(outer, env);
        CHECK(capture_names(*outer_closure) == std::set<std::string>{"x"});

        auto p_val = Value::create(3_f);
        auto inner_val = outer_closure->call({p_val});
        auto inner_closure = value_to_closure(inner_val);

        CHECK(capture_names(*inner_closure)
              == std::set<std::string>{"p", "x", "y"});
        CHECK(inner_closure->debug_capture_table().lookup("x") == x_val);
        CHECK(inner_closure->debug_capture_table().lookup("y") == y_val);
        CHECK(inner_closure->debug_capture_table().lookup("p") == p_val);

        auto out = inner_closure->call({Value::create(4_f)});
        CHECK(out->get<Int>() == 10_f);
    }

    SECTION("Nested lambda local define prevents capture")
    {
        // Frost:
        // def outer = fn () -> { fn () -> { def x = 1 ; x } }
        Symbol_Table env;

        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(node<Define>(
            AST_Node::no_range, "x",
            node<Literal>(AST_Node::no_range, Value::create(1_f))));
        inner_body.push_back(node<Name_Lookup>(AST_Node::no_range, "x"));

        std::vector<Statement::Ptr> outer_body;
        outer_body.push_back(node<Lambda>(AST_Node::no_range,
                                          std::vector<std::string>{},
                                          std::move(inner_body)));

        Lambda outer{AST_Node::no_range, {}, std::move(outer_body)};
        auto outer_closure = eval_to_closure(outer, env);
        CHECK(capture_names(*outer_closure).empty());

        auto inner_val = outer_closure->call({});
        auto inner_closure = value_to_closure(inner_val);
        CHECK(capture_names(*inner_closure).empty());

        auto out = inner_closure->call({});
        CHECK(out->get<Int>() == 1_f);
    }

    SECTION("Inner parameter shadows outer local definition")
    {
        // Frost:
        // def outer = fn () -> { def x = 1 ; fn (x) -> { x } }
        Symbol_Table env;
        auto x_outer = Value::create(1_f);

        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(node<Name_Lookup>(AST_Node::no_range, "x"));

        std::vector<Statement::Ptr> outer_body;
        outer_body.push_back(
            node<Define>(AST_Node::no_range, "x",
                         node<Literal>(AST_Node::no_range, x_outer)));
        outer_body.push_back(node<Lambda>(AST_Node::no_range,
                                          std::vector<std::string>{"x"},
                                          std::move(inner_body)));

        Lambda outer{AST_Node::no_range, {}, std::move(outer_body)};
        auto outer_closure = eval_to_closure(outer, env);
        CHECK(capture_names(*outer_closure).empty());

        auto inner_val = outer_closure->call({});
        auto inner_closure = value_to_closure(inner_val);
        CHECK(capture_names(*inner_closure).empty());

        auto arg = Value::create(2_f);
        CHECK(inner_closure->call({arg}) == arg);
    }

    SECTION("Inner parameter shadows outer parameter")
    {
        // Frost:
        // def outer = fn (x) -> { fn (x) -> { x } }
        Symbol_Table env;

        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(node<Name_Lookup>(AST_Node::no_range, "x"));

        std::vector<Statement::Ptr> outer_body;
        outer_body.push_back(node<Lambda>(AST_Node::no_range,
                                          std::vector<std::string>{"x"},
                                          std::move(inner_body)));

        Lambda outer{AST_Node::no_range, {"x"}, std::move(outer_body)};
        auto outer_closure = eval_to_closure(outer, env);
        CHECK(capture_names(*outer_closure).empty());

        auto outer_arg = Value::create(1_f);
        auto inner_val = outer_closure->call({outer_arg});
        auto inner_closure = value_to_closure(inner_val);

        CHECK(capture_names(*inner_closure).empty());
        auto inner_arg = Value::create(3_f);
        CHECK(inner_closure->call({inner_arg}) == inner_arg);
    }

    SECTION("Use-before-define allowed with later local shadow (nested params)")
    {
        // Frost:
        // def outer = fn (p) -> {
        //     fn () -> { p ; def p = 1 ; p }
        // }
        Symbol_Table env;

        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(node<Name_Lookup>(AST_Node::no_range, "p"));
        inner_body.push_back(node<Define>(
            AST_Node::no_range, "p",
            node<Literal>(AST_Node::no_range, Value::create(1_f))));
        inner_body.push_back(node<Name_Lookup>(AST_Node::no_range, "p"));

        std::vector<Statement::Ptr> outer_body;
        outer_body.push_back(node<Lambda>(AST_Node::no_range,
                                          std::vector<std::string>{},
                                          std::move(inner_body)));

        Lambda outer{AST_Node::no_range, {"p"}, std::move(outer_body)};
        auto outer_closure = eval_to_closure(outer, env);
        CHECK(capture_names(*outer_closure).empty());

        auto p_val = Value::create(5_f);
        auto inner_val = outer_closure->call({p_val});
        auto inner_closure = value_to_closure(inner_val);
        CHECK(capture_names(*inner_closure) == std::set<std::string>{"p"});
        CHECK(inner_closure->debug_capture_table().lookup("p") == p_val);

        auto out = inner_closure->call({});
        CHECK(out->get<Int>() == 1_f);
    }

    SECTION("Multiple nested lambdas capture distinct free names")
    {
        // Frost:
        // def x = 1
        // def y = 2
        // def outer = fn () -> {
        //     [ fn () -> { x }, fn () -> { y } ]
        // }
        Symbol_Table env;
        auto x_val = Value::create(1_f);
        auto y_val = Value::create(2_f);
        env.define("x", x_val);
        env.define("y", y_val);

        std::vector<Statement::Ptr> inner_x_body;
        inner_x_body.push_back(node<Name_Lookup>(AST_Node::no_range, "x"));
        std::vector<Statement::Ptr> inner_y_body;
        inner_y_body.push_back(node<Name_Lookup>(AST_Node::no_range, "y"));

        std::vector<Statement::Ptr> outer_body;
        {
            std::vector<Expression::Ptr> elems;
            elems.push_back(node<Lambda>(AST_Node::no_range,
                                         std::vector<std::string>{},
                                         std::move(inner_x_body)));
            elems.push_back(node<Lambda>(AST_Node::no_range,
                                         std::vector<std::string>{},
                                         std::move(inner_y_body)));
            outer_body.push_back(
                node<Array_Constructor>(AST_Node::no_range, std::move(elems)));
        }

        Lambda outer{AST_Node::no_range, {}, std::move(outer_body)};
        auto outer_closure = eval_to_closure(outer, env);
        CHECK(capture_names(*outer_closure) == std::set<std::string>{"x", "y"});

        auto result = outer_closure->call({});
        auto arr = result->get<Array>().value();
        REQUIRE(arr.size() == 2);
        auto first_inner = value_to_closure(arr[0]);
        auto second_inner = value_to_closure(arr[1]);

        CHECK(capture_names(*first_inner) == std::set<std::string>{"x"});
        CHECK(capture_names(*second_inner) == std::set<std::string>{"y"});
        CHECK(first_inner->call({}) == x_val);
        CHECK(second_inner->call({}) == y_val);
    }

    SECTION("Three-level nesting with parameters at each level")
    {
        // Frost:
        // def g = 1
        // def outer = fn (a) -> {
        //     def mid = fn (b) -> {
        //         fn (c) -> { g + a + b + c }
        //     }
        //     mid
        // }
        Symbol_Table env;
        auto g_val = Value::create(1_f);
        env.define("g", g_val);

        std::vector<Statement::Ptr> inner_body;
        {
            auto sum_ga = node<Binop>(
                AST_Node::no_range,
                node<Name_Lookup>(AST_Node::no_range, "g"), Binary_Op::PLUS,
                node<Name_Lookup>(AST_Node::no_range, "a"));
            auto sum_gab = node<Binop>(
                AST_Node::no_range, std::move(sum_ga), Binary_Op::PLUS,
                node<Name_Lookup>(AST_Node::no_range, "b"));
            inner_body.push_back(node<Binop>(
                AST_Node::no_range, std::move(sum_gab), Binary_Op::PLUS,
                node<Name_Lookup>(AST_Node::no_range, "c")));
        }

        std::vector<Statement::Ptr> mid_body;
        mid_body.push_back(node<Lambda>(AST_Node::no_range,
                                        std::vector<std::string>{"c"},
                                        std::move(inner_body)));

        std::vector<Statement::Ptr> outer_body;
        outer_body.push_back(node<Lambda>(AST_Node::no_range,
                                          std::vector<std::string>{"b"},
                                          std::move(mid_body)));

        Lambda outer{AST_Node::no_range, {"a"}, std::move(outer_body)};
        auto outer_closure = eval_to_closure(outer, env);
        CHECK(capture_names(*outer_closure) == std::set<std::string>{"g"});

        auto a_val = Value::create(2_f);
        auto mid_val = outer_closure->call({a_val});
        auto mid_closure = value_to_closure(mid_val);
        CHECK(capture_names(*mid_closure) == std::set<std::string>{"a", "g"});

        auto b_val = Value::create(3_f);
        auto inner_val = mid_closure->call({b_val});
        auto inner_closure = value_to_closure(inner_val);
        CHECK(capture_names(*inner_closure)
              == std::set<std::string>{"a", "b", "g"});

        auto out = inner_closure->call({Value::create(4_f)});
        CHECK(out->get<Int>() == 10_f);
    }

    SECTION("Nested lambdas in conditional branches capture union of names")
    {
        // Frost:
        // def cond = true
        // def x = 1
        // def y = 2
        // def outer = fn () -> {
        //     if cond: fn () -> { x } else: fn () -> { y }
        // }
        Symbol_Table env;
        auto cond_val = Value::create(true);
        auto x_val = Value::create(1_f);
        auto y_val = Value::create(2_f);
        env.define("cond", cond_val);
        env.define("x", x_val);
        env.define("y", y_val);

        std::vector<Statement::Ptr> inner_x_body;
        inner_x_body.push_back(node<Name_Lookup>(AST_Node::no_range, "x"));
        std::vector<Statement::Ptr> inner_y_body;
        inner_y_body.push_back(node<Name_Lookup>(AST_Node::no_range, "y"));

        std::vector<Statement::Ptr> outer_body;
        outer_body.push_back(node<If>(
            AST_Node::no_range, node<Name_Lookup>(AST_Node::no_range, "cond"),
            node<Lambda>(AST_Node::no_range, std::vector<std::string>{},
                         std::move(inner_x_body)),
            std::optional<Expression::Ptr>{
                node<Lambda>(AST_Node::no_range, std::vector<std::string>{},
                             std::move(inner_y_body))}));

        Lambda outer{AST_Node::no_range, {}, std::move(outer_body)};
        auto outer_closure = eval_to_closure(outer, env);
        CHECK(capture_names(*outer_closure)
              == std::set<std::string>{"cond", "x", "y"});

        auto inner_val = outer_closure->call({});
        auto inner_closure = value_to_closure(inner_val);
        CHECK(capture_names(*inner_closure) == std::set<std::string>{"x"});
        CHECK(inner_closure->call({}) == x_val);
    }

    SECTION("Nested lambda embedded in a larger expression")
    {
        // Frost:
        // def x = 7
        // def outer = fn () -> { [ fn () -> { x }, 0 ] }
        Symbol_Table env;
        auto x_val = Value::create(7_f);
        env.define("x", x_val);

        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(node<Name_Lookup>(AST_Node::no_range, "x"));

        std::vector<Statement::Ptr> outer_body;
        {
            std::vector<Expression::Ptr> elems;
            elems.push_back(node<Lambda>(AST_Node::no_range,
                                         std::vector<std::string>{},
                                         std::move(inner_body)));
            elems.push_back(
                node<Literal>(AST_Node::no_range, Value::create(0_f)));
            outer_body.push_back(
                node<Array_Constructor>(AST_Node::no_range, std::move(elems)));
        }

        Lambda outer{AST_Node::no_range, {}, std::move(outer_body)};
        auto outer_closure = eval_to_closure(outer, env);
        CHECK(capture_names(*outer_closure) == std::set<std::string>{"x"});

        auto arr_val = outer_closure->call({});
        auto arr = arr_val->get<Array>().value();
        REQUIRE(arr.size() == 2);
        auto inner_closure = value_to_closure(arr[0]);
        CHECK(capture_names(*inner_closure) == std::set<std::string>{"x"});
        CHECK(inner_closure->call({}) == x_val);
    }

    SECTION("Nested lambda use-before-define with later local shadow")
    {
        // Frost:
        // def y = 10
        // def outer = fn () -> { fn () -> { y ; def y = 1 ; y } }
        Symbol_Table env;
        auto y_val = Value::create(10_f);
        env.define("y", y_val);

        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(node<Name_Lookup>(AST_Node::no_range, "y"));
        inner_body.push_back(node<Define>(
            AST_Node::no_range, "y",
            node<Literal>(AST_Node::no_range, Value::create(1_f))));
        inner_body.push_back(node<Name_Lookup>(AST_Node::no_range, "y"));

        std::vector<Statement::Ptr> outer_body;
        outer_body.push_back(node<Lambda>(AST_Node::no_range,
                                          std::vector<std::string>{},
                                          std::move(inner_body)));

        Lambda outer{AST_Node::no_range, {}, std::move(outer_body)};
        auto outer_closure = eval_to_closure(outer, env);
        CHECK(capture_names(*outer_closure) == std::set<std::string>{"y"});

        auto inner_val = outer_closure->call({});
        auto inner_closure = value_to_closure(inner_val);
        CHECK(capture_names(*inner_closure) == std::set<std::string>{"y"});
        CHECK(inner_closure->debug_capture_table().lookup("y") == y_val);

        auto out = inner_closure->call({});
        CHECK(out->get<Int>() == 1_f);
    }

    SECTION("Nested lambda captures from failover chain")
    {
        // Frost:
        // def x = 5
        // def outer = fn () -> { fn () -> { x } }
        Symbol_Table outer_env;
        auto x_val = Value::create(5_f);
        outer_env.define("x", x_val);
        Symbol_Table env{&outer_env};

        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(node<Name_Lookup>(AST_Node::no_range, "x"));

        std::vector<Statement::Ptr> outer_body;
        outer_body.push_back(node<Lambda>(AST_Node::no_range,
                                          std::vector<std::string>{},
                                          std::move(inner_body)));

        Lambda outer{AST_Node::no_range, {}, std::move(outer_body)};
        auto outer_closure = eval_to_closure(outer, env);
        CHECK(capture_names(*outer_closure) == std::set<std::string>{"x"});
        CHECK(outer_closure->debug_capture_table().lookup("x") == x_val);

        auto inner_val = outer_closure->call({});
        auto inner_closure = value_to_closure(inner_val);
        CHECK(capture_names(*inner_closure) == std::set<std::string>{"x"});
        CHECK(inner_closure->debug_capture_table().lookup("x") == x_val);
        CHECK(inner_closure->call({}) == x_val);
    }

    SECTION("Multiple nested lambdas with overlapping free names")
    {
        // Frost:
        // def x = 1
        // def y = 2
        // def outer = fn () -> {
        //     [ fn () -> { x }, fn () -> { x + y }, fn () -> { y } ]
        // }
        Symbol_Table env;
        auto x_val = Value::create(1_f);
        auto y_val = Value::create(2_f);
        env.define("x", x_val);
        env.define("y", y_val);

        std::vector<Statement::Ptr> inner_x_body;
        inner_x_body.push_back(node<Name_Lookup>(AST_Node::no_range, "x"));
        std::vector<Statement::Ptr> inner_xy_body;
        inner_xy_body.push_back(node<Binop>(
            AST_Node::no_range, node<Name_Lookup>(AST_Node::no_range, "x"),
            Binary_Op::PLUS, node<Name_Lookup>(AST_Node::no_range, "y")));
        std::vector<Statement::Ptr> inner_y_body;
        inner_y_body.push_back(node<Name_Lookup>(AST_Node::no_range, "y"));

        std::vector<Statement::Ptr> outer_body;
        {
            std::vector<Expression::Ptr> elems;
            elems.push_back(node<Lambda>(AST_Node::no_range,
                                         std::vector<std::string>{},
                                         std::move(inner_x_body)));
            elems.push_back(node<Lambda>(AST_Node::no_range,
                                         std::vector<std::string>{},
                                         std::move(inner_xy_body)));
            elems.push_back(node<Lambda>(AST_Node::no_range,
                                         std::vector<std::string>{},
                                         std::move(inner_y_body)));
            outer_body.push_back(
                node<Array_Constructor>(AST_Node::no_range, std::move(elems)));
        }

        Lambda outer{AST_Node::no_range, {}, std::move(outer_body)};
        auto outer_closure = eval_to_closure(outer, env);
        CHECK(capture_names(*outer_closure) == std::set<std::string>{"x", "y"});

        auto arr_val = outer_closure->call({});
        auto arr = arr_val->get<Array>().value();
        REQUIRE(arr.size() == 3);
        auto f_x = value_to_closure(arr[0]);
        auto f_xy = value_to_closure(arr[1]);
        auto f_y = value_to_closure(arr[2]);

        CHECK(capture_names(*f_x) == std::set<std::string>{"x"});
        CHECK(capture_names(*f_xy) == std::set<std::string>{"x", "y"});
        CHECK(capture_names(*f_y) == std::set<std::string>{"y"});

        CHECK(f_x->call({}) == x_val);
        CHECK(f_y->call({}) == y_val);
        CHECK(f_xy->call({})->get<Int>() == 3_f);
    }

    SECTION("Conditional branches with parameters (true branch)")
    {
        // Frost:
        // def cond = true
        // def x = 1
        // def y = 2
        // def outer = fn (p) -> {
        //     if cond: fn (q) -> { x + p + q }
        //     else: fn (q) -> { y + p + q }
        // }
        Symbol_Table env;
        env.define("cond", Value::create(true));
        auto x_val = Value::create(1_f);
        auto y_val = Value::create(2_f);
        env.define("x", x_val);
        env.define("y", y_val);

        std::vector<Statement::Ptr> inner_x_body;
        {
            auto sum_xp = node<Binop>(
                AST_Node::no_range,
                node<Name_Lookup>(AST_Node::no_range, "x"), Binary_Op::PLUS,
                node<Name_Lookup>(AST_Node::no_range, "p"));
            inner_x_body.push_back(node<Binop>(
                AST_Node::no_range, std::move(sum_xp), Binary_Op::PLUS,
                node<Name_Lookup>(AST_Node::no_range, "q")));
        }
        std::vector<Statement::Ptr> inner_y_body;
        {
            auto sum_yp = node<Binop>(
                AST_Node::no_range,
                node<Name_Lookup>(AST_Node::no_range, "y"), Binary_Op::PLUS,
                node<Name_Lookup>(AST_Node::no_range, "p"));
            inner_y_body.push_back(node<Binop>(
                AST_Node::no_range, std::move(sum_yp), Binary_Op::PLUS,
                node<Name_Lookup>(AST_Node::no_range, "q")));
        }

        std::vector<Statement::Ptr> outer_body;
        outer_body.push_back(node<If>(
            AST_Node::no_range, node<Name_Lookup>(AST_Node::no_range, "cond"),
            node<Lambda>(AST_Node::no_range, std::vector<std::string>{"q"},
                         std::move(inner_x_body)),
            std::optional<Expression::Ptr>{
                node<Lambda>(AST_Node::no_range, std::vector<std::string>{"q"},
                             std::move(inner_y_body))}));

        Lambda outer{AST_Node::no_range, {"p"}, std::move(outer_body)};
        auto outer_closure = eval_to_closure(outer, env);
        CHECK(capture_names(*outer_closure)
              == std::set<std::string>{"cond", "x", "y"});

        auto p_val = Value::create(3_f);
        auto inner_val = outer_closure->call({p_val});
        auto inner_closure = value_to_closure(inner_val);

        CHECK(capture_names(*inner_closure) == std::set<std::string>{"p", "x"});
        auto out = inner_closure->call({Value::create(4_f)});
        CHECK(out->get<Int>() == 8_f);
    }

    SECTION("Conditional branches with parameters (false branch)")
    {
        // Frost:
        // def cond = false
        // def x = 1
        // def y = 2
        // def outer = fn (p) -> {
        //     if cond: fn (q) -> { x + p + q }
        //     else: fn (q) -> { y + p + q }
        // }
        Symbol_Table env;
        env.define("cond", Value::create(false));
        auto x_val = Value::create(1_f);
        auto y_val = Value::create(2_f);
        env.define("x", x_val);
        env.define("y", y_val);

        std::vector<Statement::Ptr> inner_x_body;
        {
            auto sum_xp = node<Binop>(
                AST_Node::no_range,
                node<Name_Lookup>(AST_Node::no_range, "x"), Binary_Op::PLUS,
                node<Name_Lookup>(AST_Node::no_range, "p"));
            inner_x_body.push_back(node<Binop>(
                AST_Node::no_range, std::move(sum_xp), Binary_Op::PLUS,
                node<Name_Lookup>(AST_Node::no_range, "q")));
        }
        std::vector<Statement::Ptr> inner_y_body;
        {
            auto sum_yp = node<Binop>(
                AST_Node::no_range,
                node<Name_Lookup>(AST_Node::no_range, "y"), Binary_Op::PLUS,
                node<Name_Lookup>(AST_Node::no_range, "p"));
            inner_y_body.push_back(node<Binop>(
                AST_Node::no_range, std::move(sum_yp), Binary_Op::PLUS,
                node<Name_Lookup>(AST_Node::no_range, "q")));
        }

        std::vector<Statement::Ptr> outer_body;
        outer_body.push_back(node<If>(
            AST_Node::no_range, node<Name_Lookup>(AST_Node::no_range, "cond"),
            node<Lambda>(AST_Node::no_range, std::vector<std::string>{"q"},
                         std::move(inner_x_body)),
            std::optional<Expression::Ptr>{
                node<Lambda>(AST_Node::no_range, std::vector<std::string>{"q"},
                             std::move(inner_y_body))}));

        Lambda outer{AST_Node::no_range, {"p"}, std::move(outer_body)};
        auto outer_closure = eval_to_closure(outer, env);
        CHECK(capture_names(*outer_closure)
              == std::set<std::string>{"cond", "x", "y"});

        auto p_val = Value::create(3_f);
        auto inner_val = outer_closure->call({p_val});
        auto inner_closure = value_to_closure(inner_val);

        CHECK(capture_names(*inner_closure) == std::set<std::string>{"p", "y"});
        auto out = inner_closure->call({Value::create(4_f)});
        CHECK(out->get<Int>() == 9_f);
    }

    SECTION("Same-name parameters shadow across levels")
    {
        // Frost:
        // def outer = fn (x) -> { fn (x) -> { fn () -> { x } } }
        Symbol_Table env;

        std::vector<Statement::Ptr> inner_body;
        inner_body.push_back(node<Name_Lookup>(AST_Node::no_range, "x"));

        std::vector<Statement::Ptr> mid_body;
        mid_body.push_back(node<Lambda>(AST_Node::no_range,
                                        std::vector<std::string>{},
                                        std::move(inner_body)));

        std::vector<Statement::Ptr> outer_body;
        outer_body.push_back(node<Lambda>(AST_Node::no_range,
                                          std::vector<std::string>{"x"},
                                          std::move(mid_body)));

        Lambda outer{AST_Node::no_range, {"x"}, std::move(outer_body)};
        auto outer_closure = eval_to_closure(outer, env);
        CHECK(capture_names(*outer_closure).empty());

        auto outer_arg = Value::create(1_f);
        auto mid_val = outer_closure->call({outer_arg});
        auto mid_closure = value_to_closure(mid_val);
        CHECK(capture_names(*mid_closure).empty());

        auto mid_arg = Value::create(2_f);
        auto inner_val = mid_closure->call({mid_arg});
        auto inner_closure = value_to_closure(inner_val);

        CHECK(capture_names(*inner_closure) == std::set<std::string>{"x"});
        CHECK(inner_closure->call({}) == mid_arg);
    }

    SECTION(
        "node_label covers all combinations of params, vararg, and self_name")
    {
        auto null_body = [] {
            std::vector<Statement::Ptr> body;
            body.push_back(node<Literal>(AST_Node::no_range, Value::null()));
            return body;
        };

        CHECK(Lambda{AST_Node::no_range, {}, null_body()}.node_label()
              == "Lambda()");
        CHECK(Lambda{AST_Node::no_range, {"x", "y"}, null_body()}.node_label()
              == "Lambda(x, y)");
        CHECK(Lambda{AST_Node::no_range, {}, null_body(), "rest"}.node_label()
              == "Lambda(...rest)");
        CHECK(
            Lambda{AST_Node::no_range, {"x"}, null_body(), "rest"}.node_label()
            == "Lambda(x, ...rest)");

        CHECK(Lambda{AST_Node::no_range, {}, null_body(), {}, "f"}.node_label()
              == "Lambda(f:)");
        CHECK(Lambda{AST_Node::no_range, {"x", "y"}, null_body(), {}, "f"}
                  .node_label()
              == "Lambda(f: x, y)");
        CHECK(Lambda{AST_Node::no_range, {}, null_body(), "rest", "f"}
                  .node_label()
              == "Lambda(f: ...rest)");
        CHECK(Lambda{AST_Node::no_range, {"x"}, null_body(), "rest", "f"}
                  .node_label()
              == "Lambda(f: x, ...rest)");
    }
}
