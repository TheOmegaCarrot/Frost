#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>
#include <catch2/trompeloeil.hpp>

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
using trompeloeil::_;

namespace
{
std::set<std::string> capture_names(const Closure& closure)
{
    std::set<std::string> names;
    for (const auto& entry : closure.debug_capture_table().debug_table())
        names.insert(entry.first);
    return names;
}

struct Flag_Statement final : Statement
{
    explicit Flag_Statement(int* count)
        : count_{count}
    {
    }

    void execute(Symbol_Table&) const override
    {
        ++(*count_);
    }

  protected:
    std::string node_label() const override
    {
        return "Flag_Statement";
    }

    std::generator<Child_Info> children() const override
    {
        co_return;
    }

  private:
    int* count_;
};

struct Uncaptured_Lookup final : Expression
{
    explicit Uncaptured_Lookup(std::string name)
        : name_{std::move(name)}
    {
    }

    [[nodiscard]] Value_Ptr evaluate(const Symbol_Table& syms) const final
    {
        return syms.lookup(name_);
    }

    std::generator<Symbol_Action> symbol_sequence() const final
    {
        co_return;
    }

  protected:
    std::string node_label() const final
    {
        return "Uncaptured_Lookup";
    }

  private:
    std::string name_;
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

    SECTION("Stores provided captures without analyzing body")
    {
        Symbol_Table captures;
        auto x_val = Value::create(10_f);
        auto y_val = Value::create(20_f);
        captures.define("x", x_val);
        captures.define("y", y_val);

        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>("missing"));

        Closure closure{{"p"}, &body, captures};

        CHECK(capture_names(closure) == std::set<std::string>{"x", "y"});
        CHECK(closure.debug_capture_table().lookup("x") == x_val);
        CHECK(closure.debug_capture_table().lookup("y") == y_val);
        CHECK_FALSE(closure.debug_capture_table().has("p"));
    }
}

TEST_CASE("Call Closure")
{
    // AI-generated test additions by Codex (GPT-5).
    // Signed: Codex (GPT-5).

    SECTION("Empty body returns null")
    {
        Symbol_Table captures;
        std::vector<Statement::Ptr> body;

        Closure closure{{}, &body, captures};

        auto result = closure.call({});
        CHECK(result->is<Null>());
    }

    SECTION("Missing parameters are bound to null")
    {
        Symbol_Table captures;
        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>("q"));

        Closure closure{{"p", "q"}, &body, captures};

        auto result = closure.call({Value::create(1_f)});
        CHECK(result->is<Null>());
    }

    SECTION("Multiple missing parameters are bound to null")
    {
        Symbol_Table captures;
        std::vector<Expression::Ptr> elems;
        elems.push_back(node<Name_Lookup>("a"));
        elems.push_back(node<Name_Lookup>("b"));
        elems.push_back(node<Name_Lookup>("c"));

        std::vector<Statement::Ptr> body;
        body.push_back(node<Array_Constructor>(std::move(elems)));

        Closure closure{{"a", "b", "c"}, &body, captures};

        auto result = closure.call({});
        auto arr = result->get<Array>().value();
        CHECK(arr.size() == 3);
        CHECK(arr[0]->is<Null>());
        CHECK(arr[1]->is<Null>());
        CHECK(arr[2]->is<Null>());
    }

    SECTION("Parameter values are passed by pointer")
    {
        Symbol_Table captures;
        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>("p"));

        auto p_val = Value::create(99_f);
        Closure closure{{"p"}, &body, captures};

        auto result = closure.call({p_val});
        CHECK(result == p_val);
    }

    SECTION("Literal returns exact pointer from last expression")
    {
        Symbol_Table captures;
        auto lit_val = Value::create(123_f);

        std::vector<Statement::Ptr> body;
        body.push_back(node<Literal>(lit_val));

        Closure closure{{}, &body, captures};

        auto result = closure.call({});
        CHECK(result == lit_val);
    }

    SECTION("Single closure can be called multiple times")
    {
        Symbol_Table captures;
        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>("p"));

        Closure closure{{"p"}, &body, captures};

        auto first = Value::create(1_f);
        auto second = Value::create(2_f);

        CHECK(closure.call({first}) == first);
        CHECK(closure.call({second}) == second);
    }

    SECTION("Evaluation order follows statement order")
    {
        Symbol_Table captures;

        auto first = std::make_unique<mock::Mock_Expression>();
        auto second = std::make_unique<mock::Mock_Expression>();
        auto third = std::make_unique<mock::Mock_Expression>();

        auto* first_ptr = first.get();
        auto* second_ptr = second.get();
        auto* third_ptr = third.get();

        auto first_val = Value::create(1_f);
        auto second_val = Value::create(2_f);
        auto third_val = Value::create(3_f);

        trompeloeil::sequence seq;
        REQUIRE_CALL(*first_ptr, evaluate(_))
            .IN_SEQUENCE(seq)
            .RETURN(first_val);
        REQUIRE_CALL(*second_ptr, evaluate(_))
            .IN_SEQUENCE(seq)
            .RETURN(second_val);
        REQUIRE_CALL(*third_ptr, evaluate(_))
            .IN_SEQUENCE(seq)
            .RETURN(third_val);

        std::vector<Statement::Ptr> body;
        body.push_back(std::move(first));
        body.push_back(std::move(second));
        body.push_back(std::move(third));

        Closure closure{{}, &body, captures};

        auto result = closure.call({});
        CHECK(result == third_val);
    }

    SECTION("Closure with local define can be called multiple times")
    {
        Symbol_Table captures;

        std::vector<Statement::Ptr> body;
        body.push_back(node<Define>("x", node<Literal>(Value::create(1_f))));
        body.push_back(node<Name_Lookup>("x"));

        Closure closure{{}, &body, captures};

        auto first = closure.call({});
        auto second = closure.call({});

        CHECK(first->get<Int>() == 1_f);
        CHECK(second->get<Int>() == 1_f);
    }

    SECTION("Local definitions do not persist across calls")
    {
        Symbol_Table captures;

        std::vector<Statement::Ptr> body;
        body.push_back(node<Define>("x", node<Name_Lookup>("p")));
        body.push_back(node<Name_Lookup>("x"));

        Closure closure{{"p"}, &body, captures};

        auto first = Value::create(10_f);
        auto second = Value::create(20_f);

        CHECK(closure.call({first}) == first);
        CHECK(closure.call({second}) == second);
    }

    SECTION("Captured value used before local define")
    {
        Symbol_Table captures;
        captures.define("x", Value::create(2_f));

        std::vector<Statement::Ptr> body;
        body.push_back(node<Define>("y", node<Name_Lookup>("x")));
        body.push_back(node<Define>("x", node<Literal>(Value::create(4_f))));
        body.push_back(node<Binop>(node<Name_Lookup>("x"), "+",
                                   node<Name_Lookup>("y")));

        Closure closure{{}, &body, captures};

        auto result = closure.call({});
        CHECK(result->get<Int>() == 6_f);
    }

    SECTION("Captures are visible during call")
    {
        Symbol_Table captures;
        auto x_val = Value::create(7_f);
        captures.define("x", x_val);

        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>("x"));

        Closure closure{{}, &body, captures};

        auto result = closure.call({});
        CHECK(result == x_val);
    }

    SECTION("Local definitions shadow captured values")
    {
        Symbol_Table captures;
        auto x_val = Value::create(1_f);
        captures.define("x", x_val);
        auto local_val = Value::create(2_f);

        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>("x"));
        body.push_back(node<Define>("x", node<Literal>(local_val)));
        body.push_back(node<Name_Lookup>("x"));

        Closure closure{{}, &body, captures};

        auto result = closure.call({});
        CHECK(result == local_val);
        CHECK(closure.debug_capture_table().lookup("x") == x_val);
    }

    SECTION("Last statement that is not an expression returns null")
    {
        Symbol_Table captures;
        std::vector<Statement::Ptr> body;
        body.push_back(node<Define>("x", node<Literal>(Value::create(1_f))));

        Closure closure{{}, &body, captures};

        auto result = closure.call({});
        CHECK(result->is<Null>());
    }

    SECTION("Last non-expression statement is executed")
    {
        Symbol_Table captures;
        int executed = 0;

        std::vector<Statement::Ptr> body;
        body.push_back(node<Literal>(Value::create(1_f)));
        body.push_back(node<Flag_Statement>(&executed));

        Closure closure{{}, &body, captures};

        auto result = closure.call({});
        CHECK(result->is<Null>());
        CHECK(executed == 1);
    }

    SECTION("Argument error prevents body execution")
    {
        Symbol_Table captures;

        auto expr = std::make_unique<mock::Mock_Expression>();
        auto* expr_ptr = expr.get();
        FORBID_CALL(*expr_ptr, evaluate(_));

        std::vector<Statement::Ptr> body;
        body.push_back(std::move(expr));

        Closure closure{{"p"}, &body, captures};

        CHECK_THROWS_WITH(
            closure.call({Value::create(1_f), Value::create(2_f)}),
            ContainsSubstring("too many arguments"));
    }

    SECTION("Too many arguments is an error")
    {
        Symbol_Table captures;
        std::vector<Statement::Ptr> body;
        body.push_back(node<Literal>(Value::create(1_f)));

        Closure closure{{"p"}, &body, captures};

        CHECK_THROWS_WITH(
            closure.call({Value::create(1_f), Value::create(2_f)}),
            ContainsSubstring("too many arguments"));
    }

    SECTION("Evaluation errors propagate")
    {
        Symbol_Table captures;
        std::vector<Statement::Ptr> body;
        body.push_back(node<Binop>(node<Literal>(Value::create(1_f)), "+",
                                   node<Literal>(Value::create(true))));

        Closure closure{{}, &body, captures};

        CHECK_THROWS_WITH(closure.call({}),
                          ContainsSubstring("Cannot add incompatible types"));
    }

    SECTION("Undefined name lookup propagates")
    {
        Symbol_Table captures;
        std::vector<Statement::Ptr> body;
        body.push_back(node<Uncaptured_Lookup>("missing"));

        Closure closure{{}, &body, captures};

        CHECK_THROWS_WITH(closure.call({}),
                          ContainsSubstring("Symbol")
                              && ContainsSubstring("missing")
                              && ContainsSubstring("not defined"));
    }
}

TEST_CASE("Debug Dump Closure")
{
    // AI-generated test additions by Codex (GPT-5).
    // Signed: Codex (GPT-5).

    SECTION("No captures")
    {
        Symbol_Table captures;

        std::vector<Statement::Ptr> body;
        body.push_back(node<Literal>(Value::create(42_f)));

        Closure closure{{}, &body, captures};

        const auto dump = closure.debug_dump();
        std::cout << dump;

        CHECK(dump == R"(<Closure>
Literal(42)
)");
    }

    SECTION("Captures and structured AST body")
    {
        Symbol_Table captures;
        captures.define("x", Value::create(1_f));
        captures.define("y", Value::create(2_f));

        std::vector<Statement::Ptr> body;
        body.push_back(node<If>(
            node<Name_Lookup>("x"),
            node<Binop>(node<Literal>(Value::create(1_f)), "+",
                        node<Name_Lookup>("y")),
            std::optional<Expression::Ptr>{
                node<Literal>(Value::create(0_f))}));

        Closure closure{{}, &body, captures};

        const auto dump = closure.debug_dump();
        std::cout << dump;

        const auto [header, body_dump] = split_header_body(dump);
        const auto capture_names_list = parse_capture_list(header);

        CHECK(header.starts_with("<Closure> (capturing: "));
        CHECK(header.ends_with(")"));
        CHECK(capture_names_list.size() == 2);
        CHECK(std::set<std::string>{capture_names_list.begin(),
                                    capture_names_list.end()}
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
