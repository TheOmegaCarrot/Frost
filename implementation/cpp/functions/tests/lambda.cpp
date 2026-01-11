#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>
#include <catch2/trompeloeil.hpp>

#include <frost/lambda.hpp>
#include <frost/mock/mock-expression.hpp>
#include <frost/symbol-table.hpp>

using namespace frst;
using namespace frst::ast;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;
using trompeloeil::_;

namespace
{
template <typename T, typename... Args>
std::unique_ptr<T> node(Args&&... args)
{
    return std::make_unique<T>(std::forward<Args>(args)...);
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

    SECTION("Evaluating the same lambda twice constructs distinct closures")
    {
        Symbol_Table env;
        env.define("x", Value::create(1_f));

        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>("x"));

        Lambda node{{}, std::move(body)};

        auto fn_val1 = node.evaluate(env);
        auto fn_val2 = node.evaluate(env);

        REQUIRE(fn_val1->is<Function>());
        REQUIRE(fn_val2->is<Function>());

        auto fn1 = fn_val1->get<Function>().value();
        auto fn2 = fn_val2->get<Function>().value();

        CHECK(fn1 != fn2);
    }

    SECTION("Closure construction errors propagate from evaluate")
    {
        Symbol_Table env;

        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>("x"));

        Lambda dup_params{{"x", "x"}, std::move(body)};

        CHECK_THROWS_WITH(dup_params.evaluate(env),
                          ContainsSubstring("duplicate"));
    }
}
