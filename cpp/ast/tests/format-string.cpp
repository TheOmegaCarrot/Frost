#include <catch2/catch_test_macros.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/ast/format-string.hpp>
#include <frost/ast/name-lookup.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <ranges>
#include <vector>

using namespace frst;
using namespace frst::ast;
using namespace std::literals;

namespace
{

using Seg = Format_String::Segment;
using Lit = Format_String::Literal_Segment;

Seg lit(std::string text)
{
    return Seg{Lit{std::move(text)}};
}

Seg interp(std::string name)
{
    return Seg{
        std::make_unique<Name_Lookup>(AST_Node::no_range, std::move(name))};
}

// Can't use initializer_list with move-only variant, so build vectors manually.
template <typename... Args>
std::vector<Seg> segs(Args&&... args)
{
    std::vector<Seg> v;
    (v.push_back(std::forward<Args>(args)), ...);
    return v;
}

Format_String::Ptr make_fmt(std::vector<Seg> segments)
{
    return std::make_unique<Format_String>(AST_Node::no_range,
                                           std::move(segments));
}

} // namespace

TEST_CASE("Format_String: literal-only produces the string unchanged")
{
    Symbol_Table syms;
    Evaluation_Context ctx{.symbols = syms};

    auto node = make_fmt(segs(lit("hello world")));
    auto result = node->evaluate(ctx);
    REQUIRE(result->is<String>());
    CHECK(result->raw_get<String>() == "hello world");
}

TEST_CASE("Format_String: single interpolation")
{
    Symbol_Table syms;
    syms.define("n", Value::create(42_f));
    Evaluation_Context ctx{.symbols = syms};

    auto node = make_fmt(segs(lit("value: "), interp("n")));
    auto result = node->evaluate(ctx);
    REQUIRE(result->is<String>());
    CHECK(result->raw_get<String>() == "value: 42");
}

TEST_CASE("Format_String: multiple interpolations")
{
    Symbol_Table syms;
    syms.define("a", Value::create(1_f));
    syms.define("b", Value::create(2_f));
    Evaluation_Context ctx{.symbols = syms};

    auto node = make_fmt(segs(interp("a"), lit(" + "), interp("b")));
    auto result = node->evaluate(ctx);
    REQUIRE(result->is<String>());
    CHECK(result->raw_get<String>() == "1 + 2");
}

TEST_CASE("Format_String: adjacent interpolations")
{
    Symbol_Table syms;
    syms.define("a", Value::create(1_f));
    syms.define("b", Value::create(2_f));
    Evaluation_Context ctx{.symbols = syms};

    auto node = make_fmt(segs(interp("a"), interp("b")));
    auto result = node->evaluate(ctx);
    REQUIRE(result->is<String>());
    CHECK(result->raw_get<String>() == "12");
}

TEST_CASE("Format_String: null values format as 'null'")
{
    Symbol_Table syms;
    syms.define("x", Value::null());
    Evaluation_Context ctx{.symbols = syms};

    auto node = make_fmt(segs(lit("value: "), interp("x")));
    auto result = node->evaluate(ctx);
    REQUIRE(result->is<String>());
    CHECK(result->raw_get<String>() == "value: null");
}

TEST_CASE("Format_String: empty segments produce empty string")
{
    Symbol_Table syms;
    Evaluation_Context ctx{.symbols = syms};

    auto node = make_fmt(segs());
    auto result = node->evaluate(ctx);
    REQUIRE(result->is<String>());
    CHECK(result->raw_get<String>().empty());
}

TEST_CASE("Format_String: symbol_sequence yields interpolation usages")
{
    auto node = make_fmt(segs(lit("a "), interp("x"), lit(" b "), interp("y")));
    auto actions = node->symbol_sequence() | std::ranges::to<std::vector>();
    REQUIRE(actions.size() == 2);

    auto* u1 = std::get_if<AST_Node::Usage>(&actions[0]);
    REQUIRE(u1);
    CHECK(u1->name == "x");

    auto* u2 = std::get_if<AST_Node::Usage>(&actions[1]);
    REQUIRE(u2);
    CHECK(u2->name == "y");
}

TEST_CASE("Format_String: literal-only has empty symbol_sequence")
{
    auto node = make_fmt(segs(lit("no interpolation")));
    auto actions = node->symbol_sequence() | std::ranges::to<std::vector>();
    CHECK(actions.empty());
}

TEST_CASE("Format_String: children shows interpolation expressions")
{
    auto node = make_fmt(segs(lit("a"), interp("x"), lit("b"), interp("y")));
    auto kids = node->children() | std::ranges::to<std::vector>();
    REQUIRE(kids.size() == 2);
    CHECK(kids[0].node->node_label() == "Name_Lookup(x)");
    CHECK(kids[1].node->node_label() == "Name_Lookup(y)");
}

TEST_CASE("Format_String: node_label")
{
    auto node = make_fmt(segs(lit("test")));
    CHECK(node->node_label() == "Format_String");
}
