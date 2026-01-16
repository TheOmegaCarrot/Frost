#include <catch2/catch_test_macros.hpp>

#include <frost/ast.hpp>
#include <frost/value.hpp>

#include <fmt/format.h>

#include <iostream>
#include <string>
#include <string_view>
#include <vector>

using namespace frst;
using namespace frst::ast;
using namespace std::literals;

#define LIFT(F)                                                                \
    []<typename... Ts>(Ts&&... args) {                                         \
        return F(std::forward<Ts>(args)...);                                   \
    }

namespace
{
std::string action_to_string(const Statement::Symbol_Action& action)
{
    return action.visit(Overload{
        [](const Statement::Definition& action) {
            return fmt::format("def:{}", action.name);
        },
        [](const Statement::Usage& action) {
            return fmt::format("use:{}", action.name);
        },
    });
}

std::vector<std::string> collect_sequence(const Statement& node)
{
    return node.symbol_sequence()
           | std::views::transform(&action_to_string)
           | std::ranges::to<std::vector>();
    ;
}

std::vector<std::string> collect_sequence(const Statement::Ptr& node)
{
    return collect_sequence(*node);
}

std::vector<std::string> collect_program_sequence(
    const std::vector<Statement::Ptr>& program)
{
    return program
           | std::views::transform(LIFT(collect_sequence))
           | std::views::join
           | std::ranges::to<std::vector>();
}

Expression::Ptr name(std::string_view n)
{
    return std::make_unique<Name_Lookup>(std::string{n});
}

Expression::Ptr lit_int(Int v)
{
    return std::make_unique<Literal>(Value::create(auto{v}));
}
} // namespace

TEST_CASE("Symbol Sequence")
{
    // AI-generated test additions by Codex (GPT-5).
    // Signed: Codex (GPT-5).

    SECTION("Literal yields no actions")
    {
        Literal node{Value::create(1_f)};
        CHECK(collect_sequence(node).empty());
    }

    SECTION("Name lookup yields usage")
    {
        Name_Lookup node{"x"};
        CHECK(collect_sequence(node) == std::vector<std::string>{"use:x"});
    }

    SECTION("Define yields RHS then definition")
    {
        Define node{"x", name("y")};
        CHECK(collect_sequence(node)
              == std::vector<std::string>{"use:y", "def:x"});
    }

    SECTION("Define with literal yields only definition")
    {
        Define node{"x", lit_int(1_f)};
        CHECK(collect_sequence(node) == std::vector<std::string>{"def:x"});
    }

    SECTION("Binary op yields left then right")
    {
        Binop node{name("a"), "+", name("b")};
        CHECK(collect_sequence(node)
              == std::vector<std::string>{"use:a", "use:b"});
    }

    SECTION("Binary op nests depth-first")
    {
        Binop node{name("a"), "+",
                   std::make_unique<Binop>(name("b"), "+", name("c"))};
        CHECK(collect_sequence(node)
              == std::vector<std::string>{"use:a", "use:b", "use:c"});
    }

    SECTION("Unary op yields operand sequence")
    {
        Unop node{name("x"), "-"};
        CHECK(collect_sequence(node) == std::vector<std::string>{"use:x"});
    }

    SECTION("Array constructor yields element order")
    {
        std::vector<Expression::Ptr> elems;
        elems.push_back(name("a"));
        elems.push_back(std::make_unique<Binop>(name("b"), "+", name("c")));
        elems.push_back(lit_int(7_f));

        Array_Constructor node{std::move(elems)};
        CHECK(collect_sequence(node)
              == std::vector<std::string>{"use:a", "use:b", "use:c"});
    }

    SECTION("Map constructor yields key then value, per pair")
    {
        std::vector<Map_Constructor::KV_Pair> pairs;
        pairs.emplace_back(name("k1"), name("v1"));
        pairs.emplace_back(
            name("k2"), std::make_unique<Binop>(name("v2"), "+", name("v3")));

        Map_Constructor node{std::move(pairs)};
        CHECK(collect_sequence(node)
              == std::vector<std::string>{"use:k1", "use:v1", "use:k2",
                                          "use:v2", "use:v3"});
    }

    SECTION("Index yields structure then index")
    {
        Index node{name("arr"), name("i")};
        CHECK(collect_sequence(node)
              == std::vector<std::string>{"use:arr", "use:i"});
    }

    SECTION("Function call yields function then args")
    {
        std::vector<Expression::Ptr> args;
        args.push_back(name("a"));
        args.push_back(std::make_unique<Binop>(name("b"), "+", name("c")));

        Function_Call node{name("fn"), std::move(args)};
        CHECK(collect_sequence(node)
              == std::vector<std::string>{"use:fn", "use:a", "use:b", "use:c"});
    }

    SECTION("Nested function call yields in order")
    {
        std::vector<Expression::Ptr> inner_args;
        inner_args.push_back(name("b"));
        inner_args.push_back(name("c"));

        auto inner_call =
            std::make_unique<Function_Call>(name("g"), std::move(inner_args));

        std::vector<Expression::Ptr> args;
        args.push_back(name("a"));
        args.push_back(std::move(inner_call));

        Function_Call node{name("f"), std::move(args)};
        CHECK(collect_sequence(node)
              == std::vector<std::string>{"use:f", "use:a", "use:g", "use:b",
                                          "use:c"});
    }

    SECTION("If yields condition, consequent, alternate (structural)")
    {
        If node{name("cond"), name("then"),
                std::optional<Expression::Ptr>{name("else")}};
        CHECK(collect_sequence(node)
              == std::vector<std::string>{"use:cond", "use:then", "use:else"});
    }

    SECTION("If without alternate omits alternate sequence")
    {
        If node{name("cond"), name("then")};
        CHECK(collect_sequence(node)
              == std::vector<std::string>{"use:cond", "use:then"});
    }

    SECTION("Define with nested expression keeps RHS order")
    {
        Define node{"x", std::make_unique<Binop>(name("y"), "+", name("z"))};
        CHECK(collect_sequence(node)
              == std::vector<std::string>{"use:y", "use:z", "def:x"});
    }

    SECTION("Program concatenates statement sequences in order")
    {
        std::vector<Statement::Ptr> program;
        program.push_back(std::make_unique<Define>(
            "x", std::make_unique<Binop>(name("a"), "+", name("b"))));
        program.push_back(std::make_unique<Binop>(name("x"), "*", name("c")));
        program.push_back(std::make_unique<Define>(
            "y",
            std::make_unique<If>(name("cond"), name("x"),
                                 std::optional<Expression::Ptr>{name("d")})));

        CHECK(collect_program_sequence(program)
              == std::vector<std::string>{"use:a", "use:b", "def:x", "use:x",
                                          "use:c", "use:cond", "use:x", "use:d",
                                          "def:y"});
    }

    SECTION("Program preserves structural order in nested expressions")
    {
        std::vector<Statement::Ptr> program;

        std::vector<Expression::Ptr> elems;
        elems.push_back(
            std::make_unique<If>(name("cond"), name("t"),
                                 std::optional<Expression::Ptr>{name("f")}));
        elems.push_back(std::make_unique<Binop>(name("b"), "+", name("c")));
        program.push_back(std::make_unique<Define>(
            "z", std::make_unique<Array_Constructor>(std::move(elems))));

        std::vector<Map_Constructor::KV_Pair> pairs;
        pairs.emplace_back(name("k"), name("v1"));
        pairs.emplace_back(
            name("k2"), std::make_unique<Binop>(name("v2"), "+", name("v3")));
        program.push_back(std::make_unique<Define>(
            "m", std::make_unique<Map_Constructor>(std::move(pairs))));

        program.push_back(std::make_unique<Index>(name("m"), name("k")));

        for (const auto& node : program)
        {
            node->debug_dump_ast(std::cout);
        }

        CHECK(collect_program_sequence(program)
              == std::vector<std::string>{"use:cond", "use:t", "use:f", "use:b",
                                          "use:c", "def:z", "use:k", "use:v1",
                                          "use:k2", "use:v2", "use:v3", "def:m",
                                          "use:m", "use:k"});
    }

    SECTION("Program includes every node type")
    {
        std::vector<Statement::Ptr> program;

        auto negate = std::make_unique<Unop>(name("a"), "-");

        auto add =
            std::make_unique<Binop>(std::move(negate), "+", lit_int(1_f));

        auto stmt_define_x = std::make_unique<Define>("x", std::move(add));

        program.push_back(std::move(stmt_define_x));

        auto cond = name("cond");

        auto then_expr = name("t");

        auto else_expr = name("f");

        auto if_expr = std::make_unique<If>(
            std::move(cond), std::move(then_expr),
            std::optional<Expression::Ptr>{std::move(else_expr)});

        std::vector<Expression::Ptr> arr_elems;

        arr_elems.push_back(name("x"));

        arr_elems.push_back(lit_int(2_f));

        arr_elems.push_back(std::move(if_expr));

        auto arr_expr =
            std::make_unique<Array_Constructor>(std::move(arr_elems));

        auto stmt_define_arr =
            std::make_unique<Define>("arr", std::move(arr_expr));

        program.push_back(std::move(stmt_define_arr));

        std::vector<Map_Constructor::KV_Pair> pairs;

        pairs.emplace_back(name("k1"), name("v1"));

        pairs.emplace_back(
            name("k2"), std::make_unique<Binop>(name("v2"), "+", name("v3")));

        auto map_expr = std::make_unique<Map_Constructor>(std::move(pairs));

        auto stmt_define_map =
            std::make_unique<Define>("map", std::move(map_expr));

        program.push_back(std::move(stmt_define_map));

        auto stmt_index = std::make_unique<Index>(name("arr"), name("i"));

        program.push_back(std::move(stmt_index));

        auto stmt_eq = std::make_unique<Binop>(name("x"), "==", name("y"));

        program.push_back(std::move(stmt_eq));

        std::vector<Expression::Ptr> call_args;

        call_args.push_back(name("arg1"));

        call_args.push_back(name("arg2"));

        auto stmt_call =
            std::make_unique<Function_Call>(name("func"), std::move(call_args));

        program.push_back(std::move(stmt_call));

        for (const auto& node : program)
        {
            node->debug_dump_ast(std::cout);
        }

        CHECK(collect_program_sequence(program)
              == std::vector<std::string>{
                  "use:a",  "def:x",   "use:x",    "use:cond", "use:t",
                  "use:f",  "def:arr", "use:k1",   "use:v1",   "use:k2",
                  "use:v2", "use:v3",  "def:map",  "use:arr",  "use:i",
                  "use:x",  "use:y",   "use:func", "use:arg1", "use:arg2",
              });
    }
}
