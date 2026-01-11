#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <set>

#include <frost/ast.hpp>
#include <frost/closure.hpp>
#include <frost/mock/mock-expression.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

using namespace frst;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;

namespace
{
std::set<std::string> capture_names(const Closure& closure)
{
    std::set<std::string> names;
    for (const auto& entry : closure.debug_capture_table().debug_table())
        names.insert(entry.first);
    return names;
}

struct Seq_Mock_Expression final : mock::Mock_Expression
{
    explicit Seq_Mock_Expression(std::vector<ast::Statement::Symbol_Action> seq)
        : seq_{std::move(seq)}
    {
    }

    std::generator<ast::Statement::Symbol_Action> symbol_sequence() const final
    {
        for (const auto& action : seq_)
            co_yield action;
    }

  private:
    std::vector<ast::Statement::Symbol_Action> seq_;
};
} // namespace

TEST_CASE("Construct Closure")
{
    // AI-generated test additions by Codex (GPT-5).
    // Signed: Codex (GPT-5).

    SECTION("Captures only free variables")
    {
        Symbol_Table env;
        env.define("x", Value::create(10_f));
        env.define("y", Value::create(20_f));

        std::vector<ast::Statement::Ptr> body;
        body.push_back(std::make_unique<ast::Binop>(
            std::make_unique<ast::Name_Lookup>("x"), "+",
            std::make_unique<ast::Name_Lookup>("y")));

        Closure closure{{}, std::move(body), env};

        CHECK(capture_names(closure) == std::set<std::string>{"x", "y"});
        CHECK(closure.debug_capture_table().lookup("x") == env.lookup("x"));
        CHECK(closure.debug_capture_table().lookup("y") == env.lookup("y"));
    }

    SECTION("Parameters are not captured")
    {
        Symbol_Table env;
        env.define("p", Value::create(1_f));
        env.define("x", Value::create(2_f));

        std::vector<ast::Statement::Ptr> body;
        body.push_back(std::make_unique<ast::Binop>(
            std::make_unique<ast::Name_Lookup>("p"), "+",
            std::make_unique<ast::Name_Lookup>("x")));

        Closure closure{{"p"}, std::move(body), env};

        CHECK(capture_names(closure) == std::set<std::string>{"x"});
        CHECK(closure.debug_capture_table().lookup("x") == env.lookup("x"));
        CHECK_FALSE(closure.debug_capture_table().has("p"));
    }

    SECTION("Use before define captures from environment")
    {
        Symbol_Table env;
        env.define("x", Value::create(5_f));

        std::vector<ast::Statement::Ptr> body;
        body.push_back(std::make_unique<ast::Binop>(
            std::make_unique<ast::Name_Lookup>("x"), "+",
            std::make_unique<ast::Literal>(Value::create(1_f))));
        body.push_back(std::make_unique<ast::Define>(
            "x", std::make_unique<ast::Literal>(Value::create(2_f))));

        Closure closure{{}, std::move(body), env};

        CHECK(capture_names(closure) == std::set<std::string>{"x"});
    }

    SECTION("Definitions shadow later usages")
    {
        Symbol_Table env;
        env.define("x", Value::create(5_f));
        env.define("y", Value::create(7_f));

        std::vector<ast::Statement::Ptr> body;
        body.push_back(std::make_unique<ast::Define>(
            "x", std::make_unique<ast::Name_Lookup>("y")));
        body.push_back(std::make_unique<ast::Name_Lookup>("x"));

        Closure closure{{}, std::move(body), env};

        CHECK(capture_names(closure) == std::set<std::string>{"y"});
        CHECK_FALSE(closure.debug_capture_table().has("x"));
    }

    SECTION("If captures condition and both branches (structural)")
    {
        Symbol_Table env;
        env.define("cond", Value::create(true));
        env.define("t", Value::create(1_f));
        env.define("f", Value::create(0_f));

        std::vector<ast::Statement::Ptr> body;
        body.push_back(std::make_unique<ast::If>(
            std::make_unique<ast::Name_Lookup>("cond"),
            std::make_unique<ast::Name_Lookup>("t"),
            std::optional<ast::Expression::Ptr>{
                std::make_unique<ast::Name_Lookup>("f")}));

        Closure closure{{}, std::move(body), env};

        CHECK(capture_names(closure)
              == std::set<std::string>{"cond", "f", "t"});
    }

    SECTION("Missing capture throws during construction")
    {
        Symbol_Table env;

        std::vector<ast::Statement::Ptr> body;
        body.push_back(std::make_unique<ast::Name_Lookup>("missing"));

        CHECK_THROWS_WITH(
            (Closure{{}, std::move(body), env}),
            ContainsSubstring("No definition found for captured symbol"));
    }

    SECTION("Mock expression sequence controls capture set")
    {
        Symbol_Table env;
        env.define("a", Value::create(1_f));
        env.define("c", Value::create(2_f));

        std::vector<ast::Statement::Symbol_Action> seq{
            ast::Statement::Usage{"a"},
            ast::Statement::Definition{"b"},
            ast::Statement::Usage{"b"},
            ast::Statement::Usage{"c"},
        };

        std::vector<ast::Statement::Ptr> body;
        body.push_back(std::make_unique<Seq_Mock_Expression>(seq));

        Closure closure{{}, std::move(body), env};

        CHECK(capture_names(closure) == std::set<std::string>{"a", "c"});
        CHECK_FALSE(closure.debug_capture_table().has("b"));
    }
}

TEST_CASE("Call Closure")
{
}

TEST_CASE("Debug Dump Closure")
{
}
