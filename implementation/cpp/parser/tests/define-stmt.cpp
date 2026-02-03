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
using namespace std::literals;

namespace
{
struct Program_Root
{
    static constexpr auto rule = lexy::dsl::p<frst::grammar::program>;
    static constexpr auto value =
        lexy::forward<std::vector<frst::ast::Statement::Ptr>>;
};

std::vector<frst::ast::Statement::Ptr> require_program(auto& result)
{
    auto program = std::move(result).value();
    return program;
}

frst::Value_Ptr evaluate_expression(const frst::ast::Statement::Ptr& statement,
                                    frst::Symbol_Table& table)
{
    auto* expr = dynamic_cast<const frst::ast::Expression*>(statement.get());
    REQUIRE(expr);
    return expr->evaluate(table);
}
} // namespace

TEST_CASE("Parser Define Statements")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        auto src = lexy::string_input(input);
        return lexy::parse<Program_Root>(src, lexy::noop);
    };

    SECTION("Single definition binds a name")
    {
        auto result = parse("def x = 42");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        program[0]->execute(table);

        auto value = table.lookup("x");
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 42_f);
    }

    SECTION("Exported definition returns a map")
    {
        auto result = parse("export def x = 42");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        auto exports = program[0]->execute(table);
        REQUIRE(exports.has_value());
        CHECK(exports->size() == 1);

        auto it = exports->find(frst::Value::create("x"s));
        REQUIRE(it != exports->end());
        REQUIRE(it->second->is<frst::Int>());
        CHECK(it->second->get<frst::Int>().value() == 42_f);
    }

    SECTION("Exported array destructure returns a map")
    {
        auto result = parse("export def [a, ...rest] = [1, 2, 3]");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        auto exports = program[0]->execute(table);
        REQUIRE(exports.has_value());
        CHECK(exports->size() == 2);

        auto it_a = exports->find(frst::Value::create("a"s));
        REQUIRE(it_a != exports->end());
        REQUIRE(it_a->second->is<frst::Int>());
        CHECK(it_a->second->get<frst::Int>().value() == 1_f);

        auto it_rest = exports->find(frst::Value::create("rest"s));
        REQUIRE(it_rest != exports->end());
        REQUIRE(it_rest->second->is<frst::Array>());
        auto rest_vals = it_rest->second->get<frst::Array>().value();
        REQUIRE(rest_vals.size() == 2);
        REQUIRE(rest_vals[0]->is<frst::Int>());
        REQUIRE(rest_vals[1]->is<frst::Int>());
        CHECK(rest_vals[0]->get<frst::Int>().value() == 2_f);
        CHECK(rest_vals[1]->get<frst::Int>().value() == 3_f);
    }

    SECTION("Map destructure binds names and missing keys to null")
    {
        auto result = parse(
            "def {foo: bar, [40+2]: answer, missing: none} = "
            "{foo: 'beep', [42]: 'life'}; [bar, answer, none]");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 2);

        frst::Symbol_Table table;
        program[0]->execute(table);
        auto value = evaluate_expression(program[1], table);

        REQUIRE(value->is<frst::Array>());
        auto arr = value->get<frst::Array>().value();
        REQUIRE(arr.size() == 3);

        REQUIRE(arr[0]->is<frst::String>());
        CHECK(arr[0]->get<frst::String>().value() == "beep");
        REQUIRE(arr[1]->is<frst::String>());
        CHECK(arr[1]->get<frst::String>().value() == "life");
        CHECK(arr[2]->is<frst::Null>());
    }

    SECTION("Exported map destructure returns a map")
    {
        auto result =
            parse("export def {foo: bar, [40+2]: answer} = "
                  "{foo: 1, [42]: 2}");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        auto exports = program[0]->execute(table);
        REQUIRE(exports.has_value());
        CHECK(exports->size() == 2);

        auto it_bar = exports->find(frst::Value::create("bar"s));
        REQUIRE(it_bar != exports->end());
        REQUIRE(it_bar->second->is<frst::Int>());
        CHECK(it_bar->second->get<frst::Int>().value() == 1_f);

        auto it_answer = exports->find(frst::Value::create("answer"s));
        REQUIRE(it_answer != exports->end());
        REQUIRE(it_answer->second->is<frst::Int>());
        CHECK(it_answer->second->get<frst::Int>().value() == 2_f);
    }

    SECTION("Map destructure allows line breaks and comments")
    {
        auto result = parse(
            "def { foo: bar, # comment\n"
            "      [1+1]: baz } = { foo: 1, [2]: 3 }; baz");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 2);

        frst::Symbol_Table table;
        program[0]->execute(table);
        auto value = evaluate_expression(program[1], table);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 3_f);
    }

    SECTION("Map destructure rejects trailing comma")
    {
        auto result = parse("def {foo: bar,} = {foo: 1}");
        REQUIRE_FALSE(result);
    }

    SECTION("Map destructure allows empty pattern")
    {
        auto result = parse("def { } = {foo: 1}");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        program[0]->execute(table);

        auto result2 = parse("export def { } = {foo: 1}");
        REQUIRE(result2);
        auto program2 = require_program(result2);
        REQUIRE(program2.size() == 1);

        auto exports = program2[0]->execute(table);
        REQUIRE(exports.has_value());
        CHECK(exports->empty());
    }

    SECTION("Map destructure allows whitespace around colon and key expr")
    {
        auto result = parse(
            "def { [1\n+2] : name } = { [3]: 7 }; name");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 2);

        frst::Symbol_Table table;
        program[0]->execute(table);
        auto value = evaluate_expression(program[1], table);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 7_f);
    }

    SECTION("Map destructure rejects unbracketed expression keys")
    {
        auto result = parse("def {(1+2): x} = { [3]: 1 }");
        REQUIRE_FALSE(result);
    }

    SECTION("Map destructure rejects missing key or binding")
    {
        auto result = parse("def {foo} = {foo: 1}");
        REQUIRE_FALSE(result);

        auto result2 = parse("def {foo:} = {foo: 1}");
        REQUIRE_FALSE(result2);

        auto result3 = parse("def {: x} = {foo: 1}");
        REQUIRE_FALSE(result3);
    }

    SECTION("Map destructure accepts underscore binding")
    {
        auto result = parse("def {foo: _} = {foo: 1}");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 1);
        auto* destructure =
            dynamic_cast<frst::ast::Map_Destructure*>(program[0].get());
        REQUIRE(destructure);
    }

    SECTION("Map destructure rejects keyword keys and bindings")
    {
        auto result = parse("def {if: x} = {if: 1}");
        REQUIRE_FALSE(result);

        auto result2 = parse("def {foo: if} = {foo: 1}");
        REQUIRE_FALSE(result2);
    }

    SECTION("Whitespace and comments around def components are allowed")
    {
        auto result = parse("def   x=1");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        program[0]->execute(table);
        auto value = table.lookup("x");
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 1_f);

        auto result2 = parse("def x = # comment\n1");
        REQUIRE_FALSE(result2);

        auto result3 = parse("def\nx = 2");
        REQUIRE_FALSE(result3);
    }

    SECTION("Identifier variants are accepted")
    {
        auto result = parse("def x1 = 1; def _x = 2; def x_2 = 3; x_2");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 4);

        frst::Symbol_Table table;
        program[0]->execute(table);
        program[1]->execute(table);
        program[2]->execute(table);
        auto value = evaluate_expression(program[3], table);

        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 3_f);
    }

    SECTION("Definitions can be followed by expressions")
    {
        auto result = parse("def x = 1; x");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 2);

        frst::Symbol_Table table;
        program[0]->execute(table);
        auto value = evaluate_expression(program[1], table);

        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 1_f);
    }

    SECTION("Definitions can depend on prior definitions")
    {
        auto result = parse("def x = 1; def y = x + 2; y");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 3);

        frst::Symbol_Table table;
        program[0]->execute(table);
        program[1]->execute(table);
        auto value = evaluate_expression(program[2], table);

        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 3_f);
    }

    SECTION("Definitions accept a variety of RHS expressions")
    {
        auto result = parse("def a = (1+2)\n"
                            "def b = f(3)\n"
                            "def c = arr[0]\n"
                            "def d = obj.key\n"
                            "d");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 5);

        frst::Symbol_Table table;

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
        };

        auto callable = std::make_shared<ConstantCallable>();
        callable->value = frst::Value::create(5_f);
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto arr_val = frst::Value::create(frst::Array{
            frst::Value::create(9_f),
        });
        table.define("arr", arr_val);

        frst::Map obj;
        obj.emplace(frst::Value::create(std::string{"key"}),
                    frst::Value::create(7_f));
        table.define("obj", frst::Value::create(std::move(obj)));

        program[0]->execute(table);
        program[1]->execute(table);
        program[2]->execute(table);
        program[3]->execute(table);
        auto value = evaluate_expression(program[4], table);

        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 7_f);
    }

    SECTION("Multiple definitions parse as separate statements")
    {
        auto result = parse("def x = 1; def y = 2");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 2);

        frst::Symbol_Table table;
        program[0]->execute(table);
        program[1]->execute(table);

        auto x = table.lookup("x");
        auto y = table.lookup("y");
        REQUIRE(x->is<frst::Int>());
        REQUIRE(y->is<frst::Int>());
        CHECK(x->get<frst::Int>().value() == 1_f);
        CHECK(y->get<frst::Int>().value() == 2_f);
    }

    SECTION("Array destructure parses as a statement")
    {
        auto result = parse("def [a, b] = 1");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 1);

        auto* destructure =
            dynamic_cast<frst::ast::Array_Destructure*>(program[0].get());
        REQUIRE(destructure);
    }

    SECTION("Array destructure supports rest and discard")
    {
        auto result = parse("def [_, ...rest] = 1");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 1);

        auto* destructure =
            dynamic_cast<frst::ast::Array_Destructure*>(program[0].get());
        REQUIRE(destructure);
    }

    SECTION("Array destructure allows whitespace around rest")
    {
        auto result = parse("def [a, ...  rest] = 1");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 1);

        auto* destructure =
            dynamic_cast<frst::ast::Array_Destructure*>(program[0].get());
        REQUIRE(destructure);

        auto result2 = parse("def [...   rest] = 1");
        REQUIRE(result2);
        auto program2 = require_program(result2);
        REQUIRE(program2.size() == 1);

        auto* destructure2 =
            dynamic_cast<frst::ast::Array_Destructure*>(program2[0].get());
        REQUIRE(destructure2);
    }

    SECTION("Array destructure allows whitespace around commas and brackets")
    {
        auto result = parse("def [ a , b ] = 1");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 1);

        auto* destructure =
            dynamic_cast<frst::ast::Array_Destructure*>(program[0].get());
        REQUIRE(destructure);
    }

    SECTION("Array destructure allows line breaks and comments")
    {
        auto result = parse("def [a,\n b] = 1");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 1);

        auto* destructure =
            dynamic_cast<frst::ast::Array_Destructure*>(program[0].get());
        REQUIRE(destructure);

        auto result2 = parse("def [a, # c\n b] = 1");
        REQUIRE(result2);
        auto program2 = require_program(result2);
        REQUIRE(program2.size() == 1);

        auto* destructure2 =
            dynamic_cast<frst::ast::Array_Destructure*>(program2[0].get());
        REQUIRE(destructure2);
    }

    SECTION("Array destructure allows line breaks around rest")
    {
        auto result = parse("def [...\n rest] = 1");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 1);

        auto* destructure =
            dynamic_cast<frst::ast::Array_Destructure*>(program[0].get());
        REQUIRE(destructure);

        auto result2 = parse("def [a, ... # c\n rest] = 1");
        REQUIRE(result2);
        auto program2 = require_program(result2);
        REQUIRE(program2.size() == 1);

        auto* destructure2 =
            dynamic_cast<frst::ast::Array_Destructure*>(program2[0].get());
        REQUIRE(destructure2);
    }

    SECTION("Array destructure allows rest-only discard")
    {
        auto result = parse("def [..._] = 1");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 1);

        auto* destructure =
            dynamic_cast<frst::ast::Array_Destructure*>(program[0].get());
        REQUIRE(destructure);
    }

    SECTION("Array destructure allows empty pattern")
    {
        auto result = parse("def [] = 1");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 1);

        auto* destructure =
            dynamic_cast<frst::ast::Array_Destructure*>(program[0].get());
        REQUIRE(destructure);
    }

    SECTION("Array destructure rejects keyword bindings")
    {
        CHECK_FALSE(parse("def [if] = 1"));
        CHECK_FALSE(parse("def [...if] = 1"));
    }

    SECTION("Definitions can use if expressions on the RHS")
    {
        auto result = parse("def x = if true: 1 else: 2; x");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 2);

        frst::Symbol_Table table;
        program[0]->execute(table);
        auto value = evaluate_expression(program[1], table);

        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 1_f);
    }

    SECTION("Definition followed by if is a separate statement")
    {
        auto result = parse("def x = 1; if true: 2 else: 3");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 2);

        frst::Symbol_Table table;
        program[0]->execute(table);
        auto value = evaluate_expression(program[1], table);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 2_f);
    }

    SECTION("Invalid def syntax fails to parse")
    {
        const std::string_view cases[] = {
            "export # comment\n def x = 1",
            "export foo",
            "def if = 1",
            "def x 1",
            "def x =",
            "def = 1",
            "def x = def y = 2",
            "def x = def",
            "def x = if",
            "1 + def x = 1",
            "f(def x = 1)",
            "if true: def x = 1 else: 2",
            "def x = (def y = 1)",
            "arr[def x = 1]",
            "obj.def",
            "(def x = 1)",
            "def x = 1 def y = def z = 3",
            "def x = 1 + def y = 2",
            "def x = if true: def y = 2 else: 3",
            "def x = (if true: def y = 2 else: 3)",
            "if true: 1 else: def x = 2",
            "if true: def x = 1",
            "if true: 1 elif false: def x = 2 else: 3",
            "def [a,] = 1",
            "def [a, b,] = 1",
            "def [...rest,] = 1",
            "def [a ...rest] = 1",
            "def [a, ...] = 1",
            "def [a, ...rest, b] = 1",
            "def [...rest, b] = 1",
            "def [, ...rest] = 1",
            "def [ , ] = 1",
        };

        for (const auto& input : cases)
        {
            CHECK_FALSE(parse(input));
        }
    }

    SECTION("Array destructure mixes with other statements")
    {
        auto result = parse("def [a, b] = [1, 2]; def x = 3; x");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 3);

        REQUIRE(dynamic_cast<frst::ast::Array_Destructure*>(program[0].get()));
        REQUIRE(dynamic_cast<frst::ast::Define*>(program[1].get()));
        REQUIRE(dynamic_cast<frst::ast::Expression*>(program[2].get()));
    }

    SECTION("Empty destructure with non-trivial RHS still parses")
    {
        auto result = parse("def [] = [1, 2]; def x = 3");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 2);

        REQUIRE(dynamic_cast<frst::ast::Array_Destructure*>(program[0].get()));
        REQUIRE(dynamic_cast<frst::ast::Define*>(program[1].get()));
    }
}
