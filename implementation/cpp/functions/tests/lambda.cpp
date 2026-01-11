#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>
#include <catch2/trompeloeil.hpp>

#include <frost/closure.hpp>
#include <frost/lambda.hpp>
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
    explicit Seq_Mock_Expression(std::vector<Statement::Symbol_Action> seq)
        : seq_{std::move(seq)}
    {
    }

    std::generator<Statement::Symbol_Action> symbol_sequence() const final
    {
        for (const auto& action : seq_)
            co_yield action;
    }

  private:
    std::vector<Statement::Symbol_Action> seq_;
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

std::shared_ptr<Closure> eval_to_closure(const Lambda& node,
                                         const Symbol_Table& env)
{
    auto result = node.evaluate(env);
    REQUIRE(result->is<Function>());
    auto fn = result->get<Function>().value();
    auto closure = std::dynamic_pointer_cast<Closure>(fn);
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
        Symbol_Table env;

        auto expr = std::make_unique<mock::Mock_Expression>();
        auto* expr_ptr = expr.get();
        FORBID_CALL(*expr_ptr, evaluate(_));

        std::vector<Statement::Ptr> body;
        body.push_back(std::move(expr));

        Lambda node{{}, std::move(body)};

        auto result = node.evaluate(env);
        CHECK(result->is<Function>());
    }

    SECTION("Symbol sequence is empty even with a body")
    {
        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>("x"));
        body.push_back(node<Define>("y", node<Literal>(Value::create(1_f))));

        Lambda node{{"p"}, std::move(body)};

        std::size_t count = 0;
        for ([[maybe_unused]] const auto& action : node.symbol_sequence())
            ++count;

        CHECK(count == 0);
    }

    SECTION("Duplicate parameters are rejected at construction")
    {
        std::vector<Statement::Ptr> body;

        CHECK_THROWS_WITH((Lambda{{"x", "x"}, std::move(body)}),
                          ContainsSubstring("duplicate"));
    }

    SECTION("Local definition cannot shadow a parameter")
    {
        std::vector<Statement::Ptr> body;
        body.push_back(node<Define>("x", node<Literal>(Value::create(2_f))));

        CHECK_THROWS_WITH((Lambda{{"x"}, std::move(body)}),
                          ContainsSubstring("parameter")
                              && ContainsSubstring("x"));
    }

    SECTION("Captures only free variables")
    {
        Symbol_Table env;
        auto x_val = Value::create(10_f);
        auto y_val = Value::create(20_f);
        env.define("x", x_val);
        env.define("y", y_val);

        std::vector<Statement::Ptr> body;
        body.push_back(
            node<Binop>(node<Name_Lookup>("x"), "+", node<Name_Lookup>("y")));

        Lambda node{{}, std::move(body)};
        auto closure = eval_to_closure(node, env);

        CHECK(capture_names(*closure) == std::set<std::string>{"x", "y"});
        CHECK(closure->debug_capture_table().lookup("x") == x_val);
        CHECK(closure->debug_capture_table().lookup("y") == y_val);
    }

    SECTION("Captures through a failover table")
    {
        Symbol_Table outer;
        auto x_val = Value::create(42_f);
        outer.define("x", x_val);

        Symbol_Table env{&outer};

        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>("x"));

        Lambda node{{}, std::move(body)};
        auto closure = eval_to_closure(node, env);

        CHECK(capture_names(*closure) == std::set<std::string>{"x"});
        CHECK(closure->debug_capture_table().lookup("x") == x_val);
    }

    SECTION("Parameters are not captured")
    {
        Symbol_Table env;
        auto p_val = Value::create(1_f);
        auto x_val = Value::create(2_f);
        env.define("p", p_val);
        env.define("x", x_val);

        std::vector<Statement::Ptr> body;
        body.push_back(
            node<Binop>(node<Name_Lookup>("p"), "+", node<Name_Lookup>("x")));

        Lambda node{{"p"}, std::move(body)};
        auto closure = eval_to_closure(node, env);

        CHECK(capture_names(*closure) == std::set<std::string>{"x"});
        CHECK(closure->debug_capture_table().lookup("x") == x_val);
        CHECK_FALSE(closure->debug_capture_table().has("p"));
    }

    SECTION("Use before define captures from environment")
    {
        Symbol_Table env;
        auto x_val = Value::create(5_f);
        env.define("x", x_val);

        std::vector<Statement::Ptr> body;
        body.push_back(node<Binop>(node<Name_Lookup>("x"), "+",
                                   node<Literal>(Value::create(1_f))));
        body.push_back(node<Define>("x", node<Literal>(Value::create(2_f))));

        Lambda node{{}, std::move(body)};
        auto closure = eval_to_closure(node, env);

        CHECK(capture_names(*closure) == std::set<std::string>{"x"});
        CHECK(closure->debug_capture_table().lookup("x") == x_val);
    }

    SECTION("Locals shadowing environment are not captured")
    {
        Symbol_Table env;
        auto x_val = Value::create(5_f);
        auto y_val = Value::create(7_f);
        env.define("x", x_val);
        env.define("y", y_val);

        std::vector<Statement::Ptr> body;
        body.push_back(node<Define>("x", node<Name_Lookup>("y")));
        body.push_back(node<Name_Lookup>("x"));

        Lambda node{{}, std::move(body)};
        auto closure = eval_to_closure(node, env);

        CHECK(capture_names(*closure) == std::set<std::string>{"y"});
        CHECK(closure->debug_capture_table().lookup("y") == y_val);
        CHECK_FALSE(closure->debug_capture_table().has("x"));
    }

    SECTION("Use before and after local define")
    {
        Symbol_Table env;
        auto x_val = Value::create(10_f);
        auto y_val = Value::create(20_f);
        env.define("x", x_val);
        env.define("y", y_val);

        std::vector<Statement::Ptr> body;
        body.push_back(
            node<Binop>(node<Name_Lookup>("x"), "+", node<Name_Lookup>("y")));
        body.push_back(node<Define>("x", node<Literal>(Value::create(3_f))));
        body.push_back(node<Name_Lookup>("x"));

        Lambda node{{}, std::move(body)};
        auto closure = eval_to_closure(node, env);

        CHECK(capture_names(*closure) == std::set<std::string>{"x", "y"});
        CHECK(closure->debug_capture_table().lookup("x") == x_val);
        CHECK(closure->debug_capture_table().lookup("y") == y_val);
    }

    SECTION("Definition without usage does not capture")
    {
        Symbol_Table env;
        auto x_val = Value::create(5_f);
        env.define("x", x_val);

        std::vector<Statement::Ptr> body;
        body.push_back(node<Define>("x", node<Literal>(Value::create(7_f))));

        Lambda node{{}, std::move(body)};
        auto closure = eval_to_closure(node, env);

        CHECK(capture_names(*closure).empty());
        CHECK_FALSE(closure->debug_capture_table().has("x"));
    }

    SECTION("Self-referential define captures prior usage in expression")
    {
        Symbol_Table env;
        auto x_val = Value::create(10_f);
        env.define("x", x_val);

        std::vector<Statement::Ptr> body;
        body.push_back(
            node<Define>("x", node<Binop>(node<Literal>(Value::create(1_f)),
                                          "+", node<Name_Lookup>("x"))));

        Lambda node{{}, std::move(body)};
        auto closure = eval_to_closure(node, env);

        CHECK(capture_names(*closure) == std::set<std::string>{"x"});
        CHECK(closure->debug_capture_table().lookup("x") == x_val);
    }

    SECTION("Parameter used only in define RHS is not captured")
    {
        Symbol_Table env;
        auto p_val = Value::create(3_f);
        env.define("p", p_val);

        std::vector<Statement::Ptr> body;
        body.push_back(
            node<Define>("x", node<Binop>(node<Name_Lookup>("p"), "+",
                                          node<Literal>(Value::create(1_f)))));

        Lambda node{{"p"}, std::move(body)};
        auto closure = eval_to_closure(node, env);

        CHECK(capture_names(*closure).empty());
        CHECK_FALSE(closure->debug_capture_table().has("p"));
    }

    SECTION("If captures condition and both branches (structural)")
    {
        Symbol_Table env;
        auto cond_val = Value::create(true);
        auto t_val = Value::create(1_f);
        auto f_val = Value::create(0_f);
        env.define("cond", cond_val);
        env.define("t", t_val);
        env.define("f", f_val);

        std::vector<Statement::Ptr> body;
        body.push_back(
            node<If>(node<Name_Lookup>("cond"), node<Name_Lookup>("t"),
                     std::optional<Expression::Ptr>{node<Name_Lookup>("f")}));

        Lambda node{{}, std::move(body)};
        auto closure = eval_to_closure(node, env);

        CHECK(capture_names(*closure)
              == std::set<std::string>{"cond", "f", "t"});
        CHECK(closure->debug_capture_table().lookup("cond") == cond_val);
        CHECK(closure->debug_capture_table().lookup("t") == t_val);
        CHECK(closure->debug_capture_table().lookup("f") == f_val);
    }

    SECTION("Missing capture throws during evaluate")
    {
        Symbol_Table env;

        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>("missing"));

        Lambda node{{}, std::move(body)};

        CHECK_THROWS_WITH(node.evaluate(env),
                          ContainsSubstring(
                              "No definition found for captured symbol"));
    }

    SECTION("Mock expression sequence controls capture set")
    {
        Symbol_Table env;
        auto a_val = Value::create(1_f);
        auto c_val = Value::create(2_f);
        env.define("a", a_val);
        env.define("c", c_val);

        std::vector<Statement::Symbol_Action> seq{
            Statement::Usage{"a"},
            Statement::Definition{"b"},
            Statement::Usage{"b"},
            Statement::Usage{"c"},
        };

        std::vector<Statement::Ptr> body;
        body.push_back(node<Seq_Mock_Expression>(std::move(seq)));

        Lambda node{{}, std::move(body)};
        auto closure = eval_to_closure(node, env);

        CHECK(capture_names(*closure) == std::set<std::string>{"a", "c"});
        CHECK(closure->debug_capture_table().lookup("a") == a_val);
        CHECK(closure->debug_capture_table().lookup("c") == c_val);
        CHECK_FALSE(closure->debug_capture_table().has("b"));
    }

    SECTION("Evaluated lambda captures environment and uses parameters")
    {
        Symbol_Table env;
        env.define("x", Value::create(2_f));

        std::vector<Statement::Ptr> body;
        body.push_back(node<Binop>(node<Name_Lookup>("x"), "+",
                                   node<Name_Lookup>("y")));

        Lambda node{{"y"}, std::move(body)};

        auto result = node.evaluate(env);
        REQUIRE(result->is<Function>());
        auto fn = result->get<Function>().value();

        auto out = fn->call({Value::create(4_f)});
        CHECK(out->get<Int>() == 6_f);
    }

    SECTION("Captures are read from the evaluation environment")
    {
        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>("x"));

        Lambda node{{}, std::move(body)};

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
        Symbol_Table env;
        env.define("x", Value::create(1_f));

        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>("x"));

        Lambda node{{}, std::move(body)};

        auto closure1 = eval_to_closure(node, env);
        auto closure2 = eval_to_closure(node, env);

        CHECK(closure1 != closure2);
    }
}
