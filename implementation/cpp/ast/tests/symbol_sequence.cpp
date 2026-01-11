#include <catch2/catch_test_macros.hpp>

#include <frost/ast.hpp>
#include <frost/value.hpp>

#include <fmt/format.h>

#include <string>
#include <string_view>
#include <vector>

using namespace frst;
using namespace frst::ast;
using namespace std::literals;

namespace
{
std::vector<std::string> collect_sequence(const Statement& node)
{
    std::vector<std::string> out;
    for (auto action : node.symbol_sequence())
    {
        out.push_back(action.visit(Overload{
            [](const Statement::Definition& action) {
                return fmt::format("def:{}", action.name);
            },
            [](const Statement::Usage& action) {
                return fmt::format("use:{}", action.name);
            },
        }));
    }
    return out;
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
        CHECK(collect_sequence(node) ==
              std::vector<std::string>{"use:y", "def:x"});
    }

    SECTION("Define with literal yields only definition")
    {
        Define node{"x", lit_int(1_f)};
        CHECK(collect_sequence(node) == std::vector<std::string>{"def:x"});
    }

    SECTION("Binary op yields left then right")
    {
        Binop node{name("a"), "+", name("b")};
        CHECK(collect_sequence(node) ==
              std::vector<std::string>{"use:a", "use:b"});
    }

    SECTION("Binary op nests depth-first")
    {
        Binop node{name("a"), "+",
                   std::make_unique<Binop>(name("b"), "+", name("c"))};
        CHECK(collect_sequence(node) ==
              std::vector<std::string>{"use:a", "use:b", "use:c"});
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
        CHECK(collect_sequence(node) ==
              std::vector<std::string>{"use:a", "use:b", "use:c"});
    }

    SECTION("Map constructor yields key then value, per pair")
    {
        std::vector<Map_Constructor::KV_Pair> pairs;
        pairs.emplace_back(name("k1"), name("v1"));
        pairs.emplace_back(
            name("k2"), std::make_unique<Binop>(name("v2"), "+", name("v3")));

        Map_Constructor node{std::move(pairs)};
        CHECK(collect_sequence(node) ==
              std::vector<std::string>{"use:k1", "use:v1", "use:k2", "use:v2",
                                       "use:v3"});
    }

    SECTION("Index yields structure then index")
    {
        Index node{name("arr"), name("i")};
        CHECK(collect_sequence(node) ==
              std::vector<std::string>{"use:arr", "use:i"});
    }

    SECTION("If yields condition, consequent, alternate (structural)")
    {
        If node{name("cond"), name("then"),
                std::optional<Expression::Ptr>{name("else")}};
        CHECK(collect_sequence(node) ==
              std::vector<std::string>{"use:cond", "use:then", "use:else"});
    }

    SECTION("If without alternate omits alternate sequence")
    {
        If node{name("cond"), name("then")};
        CHECK(collect_sequence(node) ==
              std::vector<std::string>{"use:cond", "use:then"});
    }

    SECTION("Define with nested expression keeps RHS order")
    {
        Define node{"x", std::make_unique<Binop>(name("y"), "+", name("z"))};
        CHECK(collect_sequence(node) ==
              std::vector<std::string>{"use:y", "use:z", "def:x"});
    }
}
