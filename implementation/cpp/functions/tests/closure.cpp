#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <iostream>
#include <set>
#include <string_view>
#include <utility>

#include <frost/ast.hpp>
#include <frost/closure.hpp>
#include <frost/mock/mock-expression.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

using namespace frst;
using namespace frst::ast;
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

std::pair<std::string, std::string> split_header_body(const std::string& dump)
{
    const auto pos = dump.find('\n');
    if (pos == std::string::npos)
        return {dump, ""};
    return {dump.substr(0, pos), dump.substr(pos + 1)};
}

std::vector<std::string> parse_capture_list(std::string_view header)
{
    constexpr std::string_view prefix = "<Closure> (capturing: ";
    if (!header.starts_with(prefix) || !header.ends_with(")"))
        return {};

    auto list = header.substr(prefix.size(), header.size() - prefix.size() - 1);
    std::vector<std::string> names;
    if (list.empty())
        return names;

    std::size_t start = 0;
    while (true)
    {
        const auto comma = list.find(',', start);
        names.emplace_back(list.substr(start, comma - start));
        if (comma == std::string_view::npos)
            break;
        start = comma + 1;
    }

    return names;
}
} // namespace

TEST_CASE("Construct Closure")
{
    // AI-generated test additions by Codex (GPT-5).
    // Signed: Codex (GPT-5).

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

        Closure closure{{}, std::move(body), env};

        CHECK(capture_names(closure) == std::set<std::string>{"x", "y"});
        CHECK(closure.debug_capture_table().lookup("x") == env.lookup("x"));
        CHECK(closure.debug_capture_table().lookup("y") == env.lookup("y"));
        CHECK(closure.debug_capture_table().lookup("x") == x_val);
        CHECK(closure.debug_capture_table().lookup("y") == y_val);
    }

    SECTION("Captures through a failover table")
    {
        Symbol_Table outer;
        auto x_val = Value::create(42_f);
        outer.define("x", x_val);

        Symbol_Table env{&outer};

        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>("x"));

        Closure closure{{}, std::move(body), env};

        CHECK(capture_names(closure) == std::set<std::string>{"x"});
        CHECK(closure.debug_capture_table().lookup("x") == x_val);
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

        Closure closure{{"p"}, std::move(body), env};

        CHECK(capture_names(closure) == std::set<std::string>{"x"});
        CHECK(closure.debug_capture_table().lookup("x") == env.lookup("x"));
        CHECK(closure.debug_capture_table().lookup("x") == x_val);
        CHECK_FALSE(closure.debug_capture_table().has("p"));
    }

    SECTION("Duplicate parameters are an error")
    {
        Symbol_Table env;

        std::vector<Statement::Ptr> body;

        CHECK_THROWS_WITH((Closure{{"x", "x"}, std::move(body), env}),
                          ContainsSubstring("duplicate"));
    }

    SECTION("Defining a parameter name is an error at construction")
    {
        Symbol_Table env;
        auto x_val = Value::create(1_f);
        env.define("x", x_val);

        std::vector<Statement::Ptr> body;
        body.push_back(node<Define>("x", node<Literal>(Value::create(2_f))));

        CHECK_THROWS_WITH((Closure{{"x"}, std::move(body), env}),
                          ContainsSubstring("parameter")
                              && ContainsSubstring("x"));
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

        Closure closure{{}, std::move(body), env};

        CHECK(capture_names(closure) == std::set<std::string>{"x"});
        CHECK(closure.debug_capture_table().lookup("x") == x_val);
    }

    SECTION("Locals shadowing construction environment are not captured")
    {
        Symbol_Table env;
        auto x_val = Value::create(5_f);
        auto y_val = Value::create(7_f);
        env.define("x", x_val);
        env.define("y", y_val);

        std::vector<Statement::Ptr> body;
        body.push_back(node<Define>("x", node<Name_Lookup>("y")));
        body.push_back(node<Name_Lookup>("x"));

        Closure closure{{}, std::move(body), env};

        CHECK(capture_names(closure) == std::set<std::string>{"y"});
        CHECK(closure.debug_capture_table().lookup("y") == y_val);
        CHECK_FALSE(closure.debug_capture_table().has("x"));
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

        Closure closure{{}, std::move(body), env};

        CHECK(capture_names(closure) == std::set<std::string>{"x", "y"});
        CHECK(closure.debug_capture_table().lookup("x") == x_val);
        CHECK(closure.debug_capture_table().lookup("y") == y_val);
    }

    SECTION("Multiple parameters are not captured")
    {
        Symbol_Table env;
        auto a_val = Value::create(1_f);
        auto b_val = Value::create(2_f);
        auto c_val = Value::create(3_f);
        env.define("a", a_val);
        env.define("b", b_val);
        env.define("c", c_val);

        std::vector<Statement::Ptr> body;
        body.push_back(
            node<Binop>(node<Name_Lookup>("a"), "+", node<Name_Lookup>("c")));
        body.push_back(
            node<Binop>(node<Name_Lookup>("b"), "+", node<Name_Lookup>("c")));

        Closure closure{{"a", "b"}, std::move(body), env};

        CHECK(capture_names(closure) == std::set<std::string>{"c"});
        CHECK(closure.debug_capture_table().lookup("c") == c_val);
        CHECK_FALSE(closure.debug_capture_table().has("a"));
        CHECK_FALSE(closure.debug_capture_table().has("b"));
    }

    SECTION("Definition without usage does not capture")
    {
        Symbol_Table env;
        auto x_val = Value::create(5_f);
        env.define("x", x_val);

        std::vector<Statement::Ptr> body;
        body.push_back(node<Define>("x", node<Literal>(Value::create(7_f))));

        Closure closure{{}, std::move(body), env};

        CHECK(capture_names(closure).empty());
        CHECK_FALSE(closure.debug_capture_table().has("x"));
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

        Closure closure{{}, std::move(body), env};

        CHECK(capture_names(closure) == std::set<std::string>{"x"});
        CHECK(closure.debug_capture_table().lookup("x") == x_val);
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

        Closure closure{{"p"}, std::move(body), env};

        CHECK(capture_names(closure).empty());
        CHECK_FALSE(closure.debug_capture_table().has("p"));
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

        Closure closure{{}, std::move(body), env};

        CHECK(capture_names(closure)
              == std::set<std::string>{"cond", "f", "t"});
        CHECK(closure.debug_capture_table().lookup("cond") == cond_val);
        CHECK(closure.debug_capture_table().lookup("t") == t_val);
        CHECK(closure.debug_capture_table().lookup("f") == f_val);
    }

    SECTION("Missing capture throws during construction")
    {
        Symbol_Table env;

        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>("missing"));

        CHECK_THROWS_WITH((Closure{{}, std::move(body), env}),
                          ContainsSubstring("No definition found for captured "
                                            "symbol")
                              && ContainsSubstring("missing"));
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

        Closure closure{{}, std::move(body), env};

        CHECK(capture_names(closure) == std::set<std::string>{"a", "c"});
        CHECK(closure.debug_capture_table().lookup("a") == a_val);
        CHECK(closure.debug_capture_table().lookup("c") == c_val);
        CHECK_FALSE(closure.debug_capture_table().has("b"));
    }
}

TEST_CASE("Call Closure")
{
}

TEST_CASE("Debug Dump Closure")
{
    // AI-generated test additions by Codex (GPT-5).
    // Signed: Codex (GPT-5).

    SECTION("No captures")
    {
        Symbol_Table env;

        std::vector<Statement::Ptr> body;
        body.push_back(node<Literal>(Value::create(42_f)));

        Closure closure{{}, std::move(body), env};

        const auto dump = closure.debug_dump();
        std::cout << dump;

        CHECK(dump == R"(<Closure>
Literal(42)
)");
    }

    SECTION("Captures and structured AST body")
    {
        Symbol_Table env;
        env.define("x", Value::create(1_f));
        env.define("y", Value::create(2_f));

        std::vector<Statement::Ptr> body;
        body.push_back(node<If>(
            node<Name_Lookup>("x"),
            node<Binop>(node<Literal>(Value::create(1_f)), "+",
                        node<Name_Lookup>("y")),
            std::optional<Expression::Ptr>{
                node<Literal>(Value::create(0_f))}));

        Closure closure{{}, std::move(body), env};

        const auto dump = closure.debug_dump();
        std::cout << dump;

        const auto [header, body_dump] = split_header_body(dump);
        const auto captures = parse_capture_list(header);

        CHECK(header.starts_with("<Closure> (capturing: "));
        CHECK(header.ends_with(")"));
        CHECK(captures.size() == 2);
        CHECK(std::set<std::string>{captures.begin(), captures.end()}
              == std::set<std::string>{"x", "y"});

        const auto capture_list_start =
            header.find("capturing: ") + std::string_view{"capturing: "}.size();
        const auto capture_list =
            std::string_view{header}.substr(capture_list_start,
                                            header.size() - capture_list_start
                                                - 1);
        CHECK(capture_list.find(' ') == std::string_view::npos);

        CHECK(body_dump == R"(If
├── Condition
│   └── Name_Lookup(x)
├── Consequent
│   └── Binary(+)
│       ├── LHS
│       │   └── Literal(1)
│       └── RHS
│           └── Name_Lookup(y)
└── Alternate
    └── Literal(0)
)");
    }
}
