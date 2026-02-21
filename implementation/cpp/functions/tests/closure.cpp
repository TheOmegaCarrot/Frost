#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>
#include <catch2/trompeloeil.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <iostream>
#include <memory>
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

    std::optional<frst::Map> execute(
        [[maybe_unused]] Symbol_Table&) const override
    {
        ++(*count_);
        return std::nullopt;
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

template <typename T, typename... Args>
std::shared_ptr<T> expr(Args&&... args)
{
    return std::make_shared<T>(std::forward<Args>(args)...);
}

std::shared_ptr<Expression> null_expr()
{
    return expr<Literal>(Value::null());
}

std::shared_ptr<Expression> lookup_array_expr(
    std::initializer_list<std::string_view> names)
{
    std::vector<Expression::Ptr> elems;
    elems.reserve(names.size());
    for (std::string_view name : names)
        elems.push_back(node<Name_Lookup>(std::string{name}));
    return expr<Array_Constructor>(std::move(elems));
}

std::shared_ptr<std::vector<Statement::Ptr>> make_body(
    std::vector<Statement::Ptr> body)
{
    return std::make_shared<std::vector<Statement::Ptr>>(std::move(body));
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
        auto body_ptr = make_body(std::move(body));

        Closure closure{{"p"}, body_ptr, null_expr(), captures, 0};

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
        auto body_ptr = make_body(std::move(body));

        Closure closure{{}, body_ptr, null_expr(), captures, 0};

        auto result = closure.call({});
        CHECK(result->is<Null>());
    }

    SECTION("Empty body with parameters returns null")
    {
        Symbol_Table captures;
        std::vector<Statement::Ptr> body;
        auto body_ptr = make_body(std::move(body));

        Closure closure{{"p", "q"}, body_ptr, null_expr(), captures, 0};

        auto result = closure.call({Value::create(1_f)});
        CHECK(result->is<Null>());
    }

    SECTION("Empty body still evaluates a non-null return expression")
    {
        Symbol_Table captures;
        auto out_val = Value::create(42_f);

        std::vector<Statement::Ptr> body;
        auto body_ptr = make_body(std::move(body));

        Closure closure{{}, body_ptr, expr<Literal>(out_val), captures, 0};

        auto result = closure.call({});
        CHECK(result == out_val);
    }

    SECTION("Missing parameters are bound to null")
    {
        Symbol_Table captures;
        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>("q"));
        auto body_ptr = make_body(std::move(body));

        Closure closure{{"p", "q"}, body_ptr, expr<Name_Lookup>("q"), captures,
                        0};

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
        auto body_ptr = make_body(std::move(body));

        Closure closure{
            {"a", "b", "c"},
            body_ptr,
            lookup_array_expr({"a", "b", "c"}),
            captures,
            0};

        auto result = closure.call({});
        auto arr = result->get<Array>().value();
        CHECK(arr.size() == 3);
        CHECK(arr[0]->is<Null>());
        CHECK(arr[1]->is<Null>());
        CHECK(arr[2]->is<Null>());
    }

    SECTION("Variadic parameter receives empty array when no extra args")
    {
        Symbol_Table captures;
        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>("rest"));
        auto body_ptr = make_body(std::move(body));

        Closure closure{{"p"}, body_ptr, expr<Name_Lookup>("rest"), captures, 0,
                        "rest"};

        auto result = closure.call({});
        REQUIRE(result->is<Array>());
        CHECK(result->get<Array>()->empty());
    }

    SECTION("Variadic parameter captures extra args in order")
    {
        Symbol_Table captures;
        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>("rest"));
        auto body_ptr = make_body(std::move(body));

        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto c = Value::create(3_f);

        Closure closure{{"p"}, body_ptr, expr<Name_Lookup>("rest"), captures, 0,
                        "rest"};

        auto result = closure.call({a, b, c});
        REQUIRE(result->is<Array>());
        auto arr = result->get<Array>();
        REQUIRE(arr->size() == 2);
        CHECK(arr->at(0) == b);
        CHECK(arr->at(1) == c);
    }

    SECTION("Variadic parameter coexists with missing fixed params")
    {
        Symbol_Table captures;
        std::vector<Expression::Ptr> elems;
        elems.push_back(node<Name_Lookup>("p"));
        elems.push_back(node<Name_Lookup>("rest"));

        std::vector<Statement::Ptr> body;
        body.push_back(node<Array_Constructor>(std::move(elems)));
        auto body_ptr = make_body(std::move(body));

        Closure closure{{"p"},
                        body_ptr,
                        lookup_array_expr({"p", "rest"}),
                        captures,
                        0,
                        "rest"};

        auto result = closure.call({});
        REQUIRE(result->is<Array>());
        auto arr = result->get<Array>().value();
        REQUIRE(arr.size() == 2);
        CHECK(arr[0]->is<Null>());
        REQUIRE(arr[1]->is<Array>());
        CHECK(arr[1]->get<Array>()->empty());
    }

    SECTION("Fixed parameters bind before variadic extras")
    {
        Symbol_Table captures;
        std::vector<Expression::Ptr> elems;
        elems.push_back(node<Name_Lookup>("p"));
        elems.push_back(node<Name_Lookup>("rest"));

        std::vector<Statement::Ptr> body;
        body.push_back(node<Array_Constructor>(std::move(elems)));
        auto body_ptr = make_body(std::move(body));

        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto c = Value::create(3_f);

        Closure closure{{"p"},
                        body_ptr,
                        lookup_array_expr({"p", "rest"}),
                        captures,
                        0,
                        "rest"};

        auto result = closure.call({a, b, c});
        REQUIRE(result->is<Array>());
        auto arr = result->get<Array>().value();
        REQUIRE(arr.size() == 2);
        CHECK(arr[0] == a);
        REQUIRE(arr[1]->is<Array>());
        auto rest = arr[1]->get<Array>();
        REQUIRE(rest->size() == 2);
        CHECK(rest->at(0) == b);
        CHECK(rest->at(1) == c);
    }

    SECTION("Variadic-only closure receives empty array")
    {
        Symbol_Table captures;
        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>("rest"));
        auto body_ptr = make_body(std::move(body));

        Closure closure{{}, body_ptr, expr<Name_Lookup>("rest"), captures, 0,
                        "rest"};

        auto result = closure.call({});
        REQUIRE(result->is<Array>());
        CHECK(result->get<Array>()->empty());
    }

    SECTION("Variadic-only closure captures all args")
    {
        Symbol_Table captures;
        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>("rest"));
        auto body_ptr = make_body(std::move(body));

        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto c = Value::create(3_f);

        Closure closure{{}, body_ptr, expr<Name_Lookup>("rest"), captures, 0,
                        "rest"};

        auto result = closure.call({a, b, c});
        REQUIRE(result->is<Array>());
        auto arr = result->get<Array>();
        REQUIRE(arr->size() == 3);
        CHECK(arr->at(0) == a);
        CHECK(arr->at(1) == b);
        CHECK(arr->at(2) == c);
    }

    SECTION("Vararg does not appear in capture table")
    {
        Symbol_Table captures;
        captures.define("x", Value::create(10_f));

        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>("x"));
        auto body_ptr = make_body(std::move(body));

        Closure closure{{"p"}, body_ptr, null_expr(), captures, 0, "rest"};

        const auto names = capture_names(closure);
        CHECK(names.contains("x"));
        CHECK_FALSE(names.contains("rest"));
    }

    SECTION("Variadic closure accepts many arguments")
    {
        Symbol_Table captures;
        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>("p"));
        auto body_ptr = make_body(std::move(body));

        Closure closure{{"p"}, body_ptr, expr<Name_Lookup>("p"), captures, 0,
                        "rest"};

        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto c = Value::create(3_f);

        CHECK_NOTHROW(closure.call({a, b, c}));
    }

    SECTION("Debug dump unaffected by variadic parameter")
    {
        Symbol_Table captures;
        std::vector<Statement::Ptr> body;
        body.push_back(node<Literal>(Value::create(42_f)));
        auto body_ptr = make_body(std::move(body));

        Closure closure{{"p"}, body_ptr, expr<Literal>(Value::create(42_f)),
                        captures, 0, "rest"};

        const auto dump = closure.debug_dump();
        std::cout << dump;

        CHECK(dump == R"(<Closure>
Literal(42)
)");
    }

    SECTION("Local definition cannot shadow variadic parameter")
    {
        Symbol_Table captures;
        std::vector<Statement::Ptr> body;
        body.push_back(node<Define>("rest", node<Literal>(Value::create(1_f))));
        body.push_back(node<Name_Lookup>("rest"));
        auto body_ptr = make_body(std::move(body));

        Closure closure{{}, body_ptr, expr<Name_Lookup>("rest"), captures, 0,
                        "rest"};

        CHECK_THROWS_WITH(closure.call({}),
                          "Cannot define rest as it is already defined");
    }

    SECTION("Parameter values are passed by pointer")
    {
        Symbol_Table captures;
        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>("p"));
        auto body_ptr = make_body(std::move(body));

        auto p_val = Value::create(99_f);
        Closure closure{{"p"}, body_ptr, expr<Name_Lookup>("p"), captures, 0};

        auto result = closure.call({p_val});
        CHECK(result == p_val);
    }

    SECTION("Literal returns exact pointer from last expression")
    {
        Symbol_Table captures;
        auto lit_val = Value::create(123_f);

        std::vector<Statement::Ptr> body;
        body.push_back(node<Literal>(lit_val));
        auto body_ptr = make_body(std::move(body));

        Closure closure{{}, body_ptr, expr<Literal>(lit_val), captures, 0};

        auto result = closure.call({});
        CHECK(result == lit_val);
    }

    SECTION("Single closure can be called multiple times")
    {
        Symbol_Table captures;
        std::vector<Statement::Ptr> body;
        body.push_back(node<Name_Lookup>("p"));
        auto body_ptr = make_body(std::move(body));

        Closure closure{{"p"}, body_ptr, expr<Name_Lookup>("p"), captures, 0};

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
        auto body_ptr = make_body(std::move(body));

        Closure closure{{}, body_ptr, expr<Literal>(third_val), captures, 0};

        auto result = closure.call({});
        CHECK(result == third_val);
    }

    SECTION("Return expression evaluates after all body statements")
    {
        Symbol_Table captures;

        auto body_expr = std::make_unique<mock::Mock_Expression>();
        auto* body_expr_ptr = body_expr.get();
        auto return_expr = std::make_shared<mock::Mock_Expression>();
        auto* return_expr_ptr = return_expr.get();

        auto body_val = Value::create(1_f);
        auto return_val = Value::create(2_f);

        trompeloeil::sequence seq;
        REQUIRE_CALL(*body_expr_ptr, evaluate(_))
            .IN_SEQUENCE(seq)
            .RETURN(body_val);
        REQUIRE_CALL(*return_expr_ptr, evaluate(_))
            .IN_SEQUENCE(seq)
            .RETURN(return_val);

        std::vector<Statement::Ptr> body;
        body.push_back(std::move(body_expr));
        auto body_ptr = make_body(std::move(body));

        Closure closure{{}, body_ptr, return_expr, captures, 0};

        auto result = closure.call({});
        CHECK(result == return_val);
    }

    SECTION("Closure with local define can be called multiple times")
    {
        Symbol_Table captures;

        std::vector<Statement::Ptr> body;
        body.push_back(node<Define>("x", node<Literal>(Value::create(1_f))));
        body.push_back(node<Name_Lookup>("x"));
        auto body_ptr = make_body(std::move(body));

        Closure closure{{}, body_ptr, expr<Name_Lookup>("x"), captures, 0};

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
        auto body_ptr = make_body(std::move(body));

        Closure closure{{"p"}, body_ptr, expr<Name_Lookup>("x"), captures, 0};

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
        body.push_back(node<Binop>(node<Name_Lookup>("x"), Binary_Op::PLUS,
                                   node<Name_Lookup>("y")));
        auto body_ptr = make_body(std::move(body));

        Closure closure{{},
                        body_ptr,
                        expr<Binop>(node<Name_Lookup>("x"), Binary_Op::PLUS,
                                    node<Name_Lookup>("y")),
                        captures,
                        0};

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
        auto body_ptr = make_body(std::move(body));

        Closure closure{{}, body_ptr, expr<Name_Lookup>("x"), captures, 0};

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
        auto body_ptr = make_body(std::move(body));

        Closure closure{{}, body_ptr, expr<Name_Lookup>("x"), captures, 0};

        auto result = closure.call({});
        CHECK(result == local_val);
        CHECK(closure.debug_capture_table().lookup("x") == x_val);
    }

    SECTION("Body statements execute when return expression is explicit null")
    {
        Symbol_Table captures;
        int executed = 0;

        std::vector<Statement::Ptr> body;
        body.push_back(node<Flag_Statement>(&executed));
        auto body_ptr = make_body(std::move(body));

        Closure closure{{}, body_ptr, null_expr(), captures, 0};

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
        auto body_ptr = make_body(std::move(body));

        Closure closure{{"p"}, body_ptr, null_expr(), captures, 0};

        CHECK_THROWS_WITH(
            closure.call({Value::create(1_f), Value::create(2_f)}),
            ContainsSubstring("too many arguments"));
    }

    SECTION("Argument error prevents return expression evaluation")
    {
        Symbol_Table captures;

        auto return_expr = std::make_shared<mock::Mock_Expression>();
        auto* return_expr_ptr = return_expr.get();
        FORBID_CALL(*return_expr_ptr, evaluate(_));

        std::vector<Statement::Ptr> body;
        auto body_ptr = make_body(std::move(body));

        Closure closure{{"p"}, body_ptr, return_expr, captures, 0};

        CHECK_THROWS_WITH(
            closure.call({Value::create(1_f), Value::create(2_f)}),
            ContainsSubstring("too many arguments"));
    }

    SECTION("Too many arguments is an error")
    {
        Symbol_Table captures;
        std::vector<Statement::Ptr> body;
        body.push_back(node<Literal>(Value::create(1_f)));
        auto body_ptr = make_body(std::move(body));

        Closure closure{{"p"}, body_ptr, expr<Literal>(Value::create(1_f)),
                        captures, 0};

        CHECK_THROWS_WITH(
            closure.call({Value::create(1_f), Value::create(2_f)}),
            ContainsSubstring("too many arguments"));
    }

    SECTION("Evaluation errors propagate")
    {
        Symbol_Table captures;
        std::vector<Statement::Ptr> body;
        body.push_back(node<Literal>(Value::null()));
        auto body_ptr = make_body(std::move(body));

        Closure closure{{},
                        body_ptr,
                        expr<Binop>(node<Literal>(Value::create(1_f)),
                                    Binary_Op::PLUS,
                                    node<Literal>(Value::create(true))),
                        captures,
                        0};

        CHECK_THROWS_WITH(closure.call({}),
                          ContainsSubstring("Cannot add incompatible types"));
    }

    SECTION("Undefined name lookup propagates")
    {
        Symbol_Table captures;
        std::vector<Statement::Ptr> body;
        body.push_back(node<Uncaptured_Lookup>("missing"));
        auto body_ptr = make_body(std::move(body));

        Closure closure{{}, body_ptr, expr<Uncaptured_Lookup>("missing"),
                        captures, 0};

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
        auto body_ptr = make_body(std::move(body));

        Closure closure{{}, body_ptr, expr<Literal>(Value::create(42_f)),
                        captures, 0};

        const auto dump = closure.debug_dump();
        std::cout << dump;

        CHECK(dump == R"(<Closure>
Literal(42)
)");
    }

    SECTION("Empty body debug dump")
    {
        Symbol_Table captures;
        std::vector<Statement::Ptr> body;
        auto body_ptr = make_body(std::move(body));

        Closure closure{{}, body_ptr, null_expr(), captures, 0};

        const auto dump = closure.debug_dump();
        std::cout << dump;

        CHECK(dump == "<Closure>\n");
    }

    SECTION("Captures and structured AST body")
    {
        Symbol_Table captures;
        captures.define("x", Value::create(1_f));
        captures.define("y", Value::create(2_f));

        std::vector<Statement::Ptr> body;
        body.push_back(node<If>(
            node<Name_Lookup>("x"),
            node<Binop>(node<Literal>(Value::create(1_f)), Binary_Op::PLUS,
                        node<Name_Lookup>("y")),
            std::optional<Expression::Ptr>{node<Literal>(Value::create(0_f))}));
        auto body_ptr = make_body(std::move(body));

        Closure closure{{}, body_ptr, null_expr(), captures, 0};

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
        const auto capture_list = std::string_view{header}.substr(
            capture_list_start, header.size() - capture_list_start - 1);
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
