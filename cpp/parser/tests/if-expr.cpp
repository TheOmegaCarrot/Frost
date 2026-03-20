#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/ast.hpp>
#include <frost/symbol-table.hpp>
#include <frost/testing/stringmaker-specializations.hpp>
#include <frost/value.hpp>

#include <lexy/action/parse.hpp>
#include <lexy/callback.hpp>
#include <lexy/input/string_input.hpp>

#include "../grammar.hpp"

using namespace frst::literals;

namespace
{
struct Expression_Root
{
    static constexpr auto whitespace = frst::grammar::ws;
    static constexpr auto rule =
        lexy::dsl::p<frst::grammar::expression> + lexy::dsl::eof;
    static constexpr auto value = lexy::forward<frst::ast::Expression::Ptr>;
};

frst::ast::Expression::Ptr require_expression(auto& result)
{
    auto expr = std::move(result).value();
    REQUIRE(expr);
    return expr;
}
} // namespace

TEST_CASE("Parser If Expressions")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        auto src = lexy::string_input<lexy::utf8_encoding>(input);
        frst::grammar::reset_parse_state(src);
        return lexy::parse<Expression_Root>(src, lexy::noop);
    };

    SECTION("Basic if/else evaluates")
    {
        auto result = parse("if true: 1 else: 2");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto value = expr->evaluate(ctx);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 1_f);

        auto result2 = parse("if false: 1 else: 2");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto value2 = expr2->evaluate(ctx);
        REQUIRE(value2->is<frst::Int>());
        CHECK(value2->get<frst::Int>().value() == 2_f);
    }

    SECTION("If without else returns Null when false")
    {
        auto result = parse("if false: 1");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto value = expr->evaluate(ctx);
        REQUIRE(value->is<frst::Null>());
    }

    SECTION("If without else returns consequent when true")
    {
        auto result = parse("if true: 1");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto value = expr->evaluate(ctx);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 1_f);
    }

    SECTION("Elif chains evaluate left-to-right")
    {
        auto result = parse("if false: 1 elif true: 2 else: 3");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto value = expr->evaluate(ctx);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 2_f);
    }

    SECTION("Elif chain without else can yield Null")
    {
        auto result = parse("if false: 1 elif false: 2");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto value = expr->evaluate(ctx);
        REQUIRE(value->is<frst::Null>());
    }

    SECTION("Nested if expressions associate correctly")
    {
        auto result = parse("if true: if false: 1 else: 2 else: 3");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto value = expr->evaluate(ctx);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 2_f);
    }

    SECTION("Longer elif chains evaluate correctly")
    {
        auto result = parse("if false: 1 elif false: 2 elif true: 3 else: 4");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto value = expr->evaluate(ctx);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 3_f);
    }

    SECTION("If expression binds as an atom in binary expressions")
    {
        auto result = parse("1 + if true: 2 else: 3");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto value = expr->evaluate(ctx);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 3_f);

        auto result2 = parse("if false: 1 else: 2 + 3");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto value2 = expr2->evaluate(ctx);
        REQUIRE(value2->is<frst::Int>());
        CHECK(value2->get<frst::Int>().value() == 5_f);
    }

    SECTION("If branches preserve standard precedence")
    {
        auto result = parse("if false: 1 else: 2 + 3 * 4");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto value = expr->evaluate(ctx);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 14_f);

        auto result2 = parse("1 + if true: 2 else: 3 * 4");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto value2 = expr2->evaluate(ctx);
        REQUIRE(value2->is<frst::Int>());
        CHECK(value2->get<frst::Int>().value() == 3_f);
    }

    SECTION("If branches can contain postfix expressions")
    {
        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};

        auto arr_val = frst::Value::create(frst::Array{
            frst::Value::create(10_f),
            frst::Value::create(20_f),
        });
        table.define("arr", arr_val);

        frst::Map obj;
        obj.emplace(frst::Value::create(std::string{"key"}),
                    frst::Value::create(7_f));
        table.define("obj", frst::Value::create(std::move(obj)));

        auto result = parse("if true: arr[0] else: obj.key");
        REQUIRE(result);
        auto expr = require_expression(result);
        auto value = expr->evaluate(ctx);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 10_f);
    }

    SECTION("If expression can be parenthesized and then called")
    {
        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};

        struct ConstantCallable final : frst::Callable
        {
            frst::Value_Ptr value;
            frst::Value_Ptr call(
                std::span<const frst::Value_Ptr>) const override
            {
                return value ? value : frst::Value::null();
            }

            std::string debug_dump() const override
            {
                return "<constant>";
            }
            std::string name() const override
            {
                return debug_dump();
            }
        };

        auto fn_a = std::make_shared<ConstantCallable>();
        fn_a->value = frst::Value::create(10_f);
        table.define("a", frst::Value::create(frst::Function{fn_a}));

        auto fn_b = std::make_shared<ConstantCallable>();
        fn_b->value = frst::Value::create(20_f);
        table.define("b", frst::Value::create(frst::Function{fn_b}));

        auto result = parse("(if true: a else: b)(1)");
        REQUIRE(result);
        auto expr = require_expression(result);
        auto value = expr->evaluate(ctx);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 10_f);
    }

    SECTION("If condition uses boolean coercion")
    {
        auto result = parse("if 0: 1 else: 2");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto value = expr->evaluate(ctx);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 1_f);

        auto result2 = parse("if null: 1 else: 2");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto value2 = expr2->evaluate(ctx);
        REQUIRE(value2->is<frst::Int>());
        CHECK(value2->get<frst::Int>().value() == 2_f);
    }

    SECTION("Whitespace and comments around colons and keywords")
    {
        auto result = parse("if true : 1 else : 2");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto value = expr->evaluate(ctx);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 1_f);

        auto result2 = parse("if true: # c\n1 else: 2");
        REQUIRE(result2);
        auto expr2 = require_expression(result2);
        auto value2 = expr2->evaluate(ctx);
        REQUIRE(value2->is<frst::Int>());
        CHECK(value2->get<frst::Int>().value() == 1_f);

        auto result3 = parse("if true:\n1");
        REQUIRE(result3);
        auto expr3 = require_expression(result3);
        auto value3 = expr3->evaluate(ctx);
        REQUIRE(value3->is<frst::Int>());
        CHECK(value3->get<frst::Int>().value() == 1_f);

        auto result4 = parse("if false:\n1\nelse:\n2");
        REQUIRE(result4);
        auto expr4 = require_expression(result4);
        auto value4 = expr4->evaluate(ctx);
        REQUIRE(value4->is<frst::Int>());
        CHECK(value4->get<frst::Int>().value() == 2_f);

        auto result5 = parse("if true:\n1\nelif true:\n2 else: 3");
        REQUIRE(result5);
        auto expr5 = require_expression(result5);
        auto value5 = expr5->evaluate(ctx);
        REQUIRE(value5->is<frst::Int>());
        CHECK(value5->get<frst::Int>().value() == 1_f);

        auto result6 = parse("if true: 1 else:\n2");
        REQUIRE(result6);
        auto expr6 = require_expression(result6);
        auto value6 = expr6->evaluate(ctx);
        REQUIRE(value6->is<frst::Int>());
        CHECK(value6->get<frst::Int>().value() == 1_f);
    }

    SECTION("Keyword prefixes in branches are treated as identifiers")
    {
        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};

        auto elifx = frst::Value::create(7_f);
        auto elsey = frst::Value::create(9_f);
        table.define("elifx", elifx);
        table.define("elsey", elsey);

        auto result = parse("if true: elifx else: elsey");
        REQUIRE(result);
        auto expr = require_expression(result);
        auto value = expr->evaluate(ctx);
        CHECK(value == elifx);
    }

    SECTION("Invalid if syntax fails to parse")
    {
        const std::string_view cases[] = {
            "if true 1",
            "if true: 1 else 2",
            "if true: 1 else:",
            "if true: 1 elif: 2",
            "if true: 1 elif false 2",
            "if true: 1 else: 2 else: 3",
            "if true: 1 else: 2 elif false: 3",
            "if true:",
            "if true: 1 elif false:",
            "elif true: 1",
            "else: 1",
            "if : 1",
            "if true: 1 else: 2 3",
        };

        for (const auto& input : cases)
        {
            CHECK_FALSE(parse(input));
        }
    }

    SECTION("Source ranges for if/else")
    {
        // "if true: 1 else: 2" → begin at 'i' {1,1}, end at '2' {1,18}
        auto result = parse("if true: 1 else: 2");
        REQUIRE(result);
        auto expr = require_expression(result);
        auto range = expr->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 1);
        CHECK(range.end.line == 1);
        CHECK(range.end.column == 18);
    }

    SECTION("Source ranges for if without else")
    {
        // "if true: 42" → begin at 'i' {1,1}, end at '2' in 42 {1,11}
        auto result = parse("if true: 42");
        REQUIRE(result);
        auto expr = require_expression(result);
        auto range = expr->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 1);
        CHECK(range.end.line == 1);
        CHECK(range.end.column == 11);
    }

    SECTION("Source ranges for if/elif/else")
    {
        // "if false: 1 elif true: 2 else: 3"
        // begin at 'i' {1,1}, end at '3' {1,32}
        auto result = parse("if false: 1 elif true: 2 else: 3");
        REQUIRE(result);
        auto expr = require_expression(result);
        auto range = expr->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 1);
        CHECK(range.end.line == 1);
        CHECK(range.end.column == 32);
    }

    SECTION("Synthetic elif If node gets source range from elif to end")
    {
        // "if a: b elif c: d else: e"
        //  ^                        ^  outer If: [1:1-1:25]
        //          ^                 ^  inner If: [1:9-1:25]
        auto result = parse("if a: b elif c: d else: e");
        REQUIRE(result);
        auto expr = require_expression(result);

        auto nodes = expr->walk() | std::ranges::to<std::vector>();
        // outer If -> Name_Lookup(a), Name_Lookup(b), inner If -> ...
        // The inner If is nodes[3] (after outer If, a, b)
        REQUIRE(nodes.size() >= 4);
        auto inner_range = nodes[3]->source_range();
        CHECK(inner_range.begin.line == 1);
        CHECK(inner_range.begin.column == 9);
        CHECK(inner_range.end.line == 1);
        CHECK(inner_range.end.column == 25);
    }

    SECTION("Chained elif nodes each get correct source ranges")
    {
        // "if a: b elif c: d elif e: f else: g"
        //  ^                                   ^  outer: [1:1-1:35]
        //          ^                            ^  elif1: [1:9-1:35]
        //                    ^                  ^  elif2: [1:19-1:35]
        auto result = parse("if a: b elif c: d elif e: f else: g");
        REQUIRE(result);
        auto expr = require_expression(result);

        auto nodes = expr->walk() | std::ranges::to<std::vector>();
        // Find the If nodes (indices 0, 3, 7 based on tree structure)
        std::vector<const frst::ast::Statement*> if_nodes;
        for (auto* n : nodes)
            if (n->node_label() == "If")
                if_nodes.push_back(n);
        REQUIRE(if_nodes.size() == 3);

        CHECK(if_nodes[0]->source_range().begin.column == 1);
        CHECK(if_nodes[0]->source_range().end.column == 35);

        CHECK(if_nodes[1]->source_range().begin.column == 9);
        CHECK(if_nodes[1]->source_range().end.column == 35);

        CHECK(if_nodes[2]->source_range().begin.column == 19);
        CHECK(if_nodes[2]->source_range().end.column == 35);
    }

    SECTION("elif without else gets range ending at consequent")
    {
        // "if a: b elif c: d"
        //          ^       ^  inner If: [1:9-1:17]
        auto result = parse("if a: b elif c: d");
        REQUIRE(result);
        auto expr = require_expression(result);

        auto nodes = expr->walk() | std::ranges::to<std::vector>();
        std::vector<const frst::ast::Statement*> if_nodes;
        for (auto* n : nodes)
            if (n->node_label() == "If")
                if_nodes.push_back(n);
        REQUIRE(if_nodes.size() == 2);

        CHECK(if_nodes[1]->source_range().begin.column == 9);
        CHECK(if_nodes[1]->source_range().end.column == 17);
    }

    SECTION("Multiline if/elif/else source ranges")
    {
        // "if a: b\nelif c: d\nelse: e"
        // outer If: [1:1-3:7], inner If (elif): [2:1-3:7]
        auto result = parse("if a: b\nelif c: d\nelse: e");
        REQUIRE(result);
        auto expr = require_expression(result);

        auto range = expr->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 1);
        CHECK(range.end.line == 3);
        CHECK(range.end.column == 7);

        auto nodes = expr->walk() | std::ranges::to<std::vector>();
        std::vector<const frst::ast::Statement*> if_nodes;
        for (auto* n : nodes)
            if (n->node_label() == "If")
                if_nodes.push_back(n);
        REQUIRE(if_nodes.size() == 2);

        CHECK(if_nodes[1]->source_range().begin.line == 2);
        CHECK(if_nodes[1]->source_range().begin.column == 1);
        CHECK(if_nodes[1]->source_range().end.line == 3);
        CHECK(if_nodes[1]->source_range().end.column == 7);
    }
}
