#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

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

frst::Value_Ptr evaluate_statement(const frst::ast::Statement::Ptr& statement,
                                   frst::Symbol_Table& table)
{
    auto* expr = dynamic_cast<const frst::ast::Expression*>(statement.get());
    REQUIRE(expr);
    return expr->evaluate(table);
}

struct Constant_Callable final : frst::Callable
{
    frst::Value_Ptr result;

    frst::Value_Ptr call(std::span<const frst::Value_Ptr>) const override
    {
        return result ? result : frst::Value::null();
    }

    std::string debug_dump() const override
    {
        return "<constant>";
    }
};

struct IdentityCallable final : frst::Callable
{
    frst::Value_Ptr call(std::span<const frst::Value_Ptr> args) const override
    {
        if (args.empty())
        {
            return frst::Value::null();
        }
        return args.front();
    }

    std::string debug_dump() const override
    {
        return "<identity>";
    }
};

struct CountingCallable final : frst::Callable
{
    mutable std::vector<frst::Value_Ptr> args;

    frst::Value_Ptr call(
        std::span<const frst::Value_Ptr> call_args) const override
    {
        if (!call_args.empty())
        {
            args.push_back(call_args.front());
        }
        return frst::Value::null();
    }

    std::string debug_dump() const override
    {
        return "<counting>";
    }
};
} // namespace

TEST_CASE("Parser Program")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        auto src = lexy::string_input(input);
        return lexy::parse<Program_Root>(src, lexy::noop);
    };

    SECTION("Empty input yields empty program")
    {
        auto result = parse("");
        REQUIRE(result);
        auto program = require_program(result);
        CHECK(program.empty());
    }

    SECTION("Only semicolons yields empty program")
    {
        const std::string_view cases[] = {";", ";;", ";;;", ";;;\n;;"};

        for (const auto& input : cases)
        {
            auto result = parse(input);
            REQUIRE(result);
            auto program = require_program(result);
            CHECK(program.empty());
        }
    }

    SECTION("Only semicolons with whitespace and comments yields empty program")
    {
        const std::string_view cases[] = {
            " ; # comment\n; ",
            "\n;\n# comment\n;;\n",
        };

        for (const auto& input : cases)
        {
            auto result = parse(input);
            REQUIRE(result);
            auto program = require_program(result);
            CHECK(program.empty());
        }
    }

    SECTION("Multiple statements with semicolons")
    {
        auto result = parse("1;2;3");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 3);

        frst::Symbol_Table table;
        auto v1 = evaluate_statement(program[0], table);
        auto v2 = evaluate_statement(program[1], table);
        auto v3 = evaluate_statement(program[2], table);

        REQUIRE(v1->is<frst::Int>());
        REQUIRE(v2->is<frst::Int>());
        REQUIRE(v3->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 1_f);
        CHECK(v2->get<frst::Int>().value() == 2_f);
        CHECK(v3->get<frst::Int>().value() == 3_f);
    }

    SECTION("Trailing semicolon after a parenthesized expression")
    {
        auto result = parse("(1);");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        auto value = evaluate_statement(program[0], table);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 1_f);
    }

    SECTION("Multiple semicolons are ignored")
    {
        auto result = parse(";1;;2;;;3;\n;;");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 3);

        frst::Symbol_Table table;
        auto v1 = evaluate_statement(program[0], table);
        auto v2 = evaluate_statement(program[1], table);
        auto v3 = evaluate_statement(program[2], table);

        REQUIRE(v1->is<frst::Int>());
        REQUIRE(v2->is<frst::Int>());
        REQUIRE(v3->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 1_f);
        CHECK(v2->get<frst::Int>().value() == 2_f);
        CHECK(v3->get<frst::Int>().value() == 3_f);
    }

    SECTION("Empty statements interleaved with comments are ignored")
    {
        auto result = parse("; # comment\n; 1");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        auto value = evaluate_statement(program[0], table);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 1_f);
    }

    SECTION("Statements can be adjacent without semicolons")
    {
        auto result = parse("1 2 3");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 3);

        frst::Symbol_Table table;
        auto v1 = evaluate_statement(program[0], table);
        auto v2 = evaluate_statement(program[1], table);
        auto v3 = evaluate_statement(program[2], table);

        REQUIRE(v1->is<frst::Int>());
        REQUIRE(v2->is<frst::Int>());
        REQUIRE(v3->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 1_f);
        CHECK(v2->get<frst::Int>().value() == 2_f);
        CHECK(v3->get<frst::Int>().value() == 3_f);
    }

    SECTION("Newlines separate statements")
    {
        auto result = parse("some_name\n42");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 2);

        frst::Symbol_Table table;
        auto name_value = frst::Value::create(7_f);
        table.define("some_name", name_value);

        auto v1 = evaluate_statement(program[0], table);
        auto v2 = evaluate_statement(program[1], table);

        CHECK(v1 == name_value);
        REQUIRE(v2->is<frst::Int>());
        CHECK(v2->get<frst::Int>().value() == 42_f);
    }

    SECTION("Mixed literal types in a single program")
    {
        auto result = parse("1; \"s\"; 't'; true; null");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 5);

        frst::Symbol_Table table;
        auto v1 = evaluate_statement(program[0], table);
        auto v2 = evaluate_statement(program[1], table);
        auto v3 = evaluate_statement(program[2], table);
        auto v4 = evaluate_statement(program[3], table);
        auto v5 = evaluate_statement(program[4], table);

        REQUIRE(v1->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 1_f);
        REQUIRE(v2->is<frst::String>());
        CHECK(v2->get<frst::String>().value() == "s");
        REQUIRE(v3->is<frst::String>());
        CHECK(v3->get<frst::String>().value() == "t");
        REQUIRE(v4->is<frst::Bool>());
        CHECK(v4->get<frst::Bool>().value() == true);
        REQUIRE(v5->is<frst::Null>());
    }

    SECTION("Array literals in a program")
    {
        auto result = parse("[]; [1, 2]; [1, 2,]; [1, 2][0]");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 4);

        frst::Symbol_Table table;
        auto v1 = evaluate_statement(program[0], table);
        auto v2 = evaluate_statement(program[1], table);
        auto v3 = evaluate_statement(program[2], table);
        auto v4 = evaluate_statement(program[3], table);

        REQUIRE(v1->is<frst::Array>());
        CHECK(v1->raw_get<frst::Array>().empty());

        REQUIRE(v2->is<frst::Array>());
        CHECK(v2->raw_get<frst::Array>().size() == 2);

        REQUIRE(v3->is<frst::Array>());
        CHECK(v3->raw_get<frst::Array>().size() == 2);

        REQUIRE(v4->is<frst::Int>());
        CHECK(v4->get<frst::Int>().value() == 1_f);
    }

    SECTION("Map literals in a program")
    {
        auto result = parse("%{a: 1}; %{[2]: 3}; %{a: 1, [2]: 3,}");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 3);

        frst::Symbol_Table table;
        auto v1 = evaluate_statement(program[0], table);
        auto v2 = evaluate_statement(program[1], table);
        auto v3 = evaluate_statement(program[2], table);

        REQUIRE(v1->is<frst::Map>());
        REQUIRE(v2->is<frst::Map>());
        REQUIRE(v3->is<frst::Map>());
    }

    SECTION("Lambda expressions and calls in a program")
    {
        auto result =
            parse("fn () -> { 1 }(); fn (x) -> { x }(3); fn () -> { 5 }");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 3);

        frst::Symbol_Table table;
        auto v1 = evaluate_statement(program[0], table);
        auto v2 = evaluate_statement(program[1], table);
        auto v3 = evaluate_statement(program[2], table);

        REQUIRE(v1->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 1_f);
        REQUIRE(v2->is<frst::Int>());
        CHECK(v2->get<frst::Int>().value() == 3_f);
        REQUIRE(v3->is<frst::Function>());
    }

    SECTION("Statements can follow lambda expressions without separators")
    {
        auto result = parse("fn () -> { 1 } 2");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 2);

        frst::Symbol_Table table;
        auto v2 = evaluate_statement(program[1], table);
        REQUIRE(v2->is<frst::Int>());
        CHECK(v2->get<frst::Int>().value() == 2_f);
    }

    SECTION("Definitions with complex right-hand sides")
    {
        auto result = parse("def a = if cond: 10 else: 20\n"
                            "def b = [1, 2][0]\n"
                            "def c = %{k: 5}.k\n"
                            "def d = fn (x) -> { x + 1 }(4)\n"
                            "a b c d");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 8);

        frst::Symbol_Table table;
        table.define("cond", frst::Value::create(true));

        program[0]->execute(table);
        program[1]->execute(table);
        program[2]->execute(table);
        program[3]->execute(table);

        auto v1 = evaluate_statement(program[4], table);
        auto v2 = evaluate_statement(program[5], table);
        auto v3 = evaluate_statement(program[6], table);
        auto v4 = evaluate_statement(program[7], table);

        REQUIRE(v1->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 10_f);
        REQUIRE(v2->is<frst::Int>());
        CHECK(v2->get<frst::Int>().value() == 1_f);
        REQUIRE(v3->is<frst::Int>());
        CHECK(v3->get<frst::Int>().value() == 5_f);
        REQUIRE(v4->is<frst::Int>());
        CHECK(v4->get<frst::Int>().value() == 5_f);
    }

    SECTION("Definitions without semicolons are separated by keywords")
    {
        auto result = parse("def x = 1 def y = 2 x y");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 4);

        frst::Symbol_Table table;
        program[0]->execute(table);
        program[1]->execute(table);
        auto v1 = evaluate_statement(program[2], table);
        auto v2 = evaluate_statement(program[3], table);

        REQUIRE(v1->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 1_f);
        REQUIRE(v2->is<frst::Int>());
        CHECK(v2->get<frst::Int>().value() == 2_f);
    }

    SECTION("If expressions can contain calls and indexing in branches")
    {
        auto result =
            parse("if a: f(1) else: arr[0]\nif false: arr[1] else: f(2)");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 2);

        frst::Symbol_Table table;
        table.define("a", frst::Value::create(false));
        table.define("arr", frst::Value::create(frst::Array{
                                frst::Value::create(5_f),
                                frst::Value::create(6_f),
                            }));

        auto f_callable = std::make_shared<Constant_Callable>();
        f_callable->result = frst::Value::create(9_f);
        table.define("f", frst::Value::create(frst::Function{f_callable}));

        auto v1 = evaluate_statement(program[0], table);
        auto v2 = evaluate_statement(program[1], table);

        REQUIRE(v1->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 5_f);
        REQUIRE(v2->is<frst::Int>());
        CHECK(v2->get<frst::Int>().value() == 9_f);
    }

    SECTION("Map literals can be postfixed in programs")
    {
        auto result = parse("%{a: 1}[\"a\"]; (%{b: 2}).b");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 2);

        frst::Symbol_Table table;
        auto v1 = evaluate_statement(program[0], table);
        auto v2 = evaluate_statement(program[1], table);

        REQUIRE(v1->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 1_f);
        REQUIRE(v2->is<frst::Int>());
        CHECK(v2->get<frst::Int>().value() == 2_f);
    }

    SECTION("UFCS expressions can appear in programs")
    {
        auto result = parse("a @ f(1); 2 @ f(); b @ obj.m()");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 3);

        frst::Symbol_Table table;
        auto a_val = frst::Value::create(10_f);
        auto b_val = frst::Value::create(20_f);
        table.define("a", a_val);
        table.define("b", b_val);

        auto callable = std::make_shared<IdentityCallable>();
        table.define("f", frst::Value::create(frst::Function{callable}));

        frst::Map obj;
        obj.emplace(frst::Value::create(std::string{"m"}),
                    frst::Value::create(frst::Function{callable}));
        table.define("obj", frst::Value::create(std::move(obj)));

        auto v1 = evaluate_statement(program[0], table);
        auto v2 = evaluate_statement(program[1], table);
        auto v3 = evaluate_statement(program[2], table);

        CHECK(v1 == a_val);
        REQUIRE(v2->is<frst::Int>());
        CHECK(v2->get<frst::Int>().value() == 2_f);
        CHECK(v3 == b_val);
    }

    SECTION("Map, filter, reduce, and foreach expressions in a program")
    {
        auto result = parse("map [1, 2, 3] with fn (x) -> { x + 1 };"
                            "filter [1, 2, 3] with fn (x) -> { x > 1 };"
                            "reduce [1, 2, 3] with fn (acc, x) -> { acc + x };"
                            "foreach [1, 2] with f");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 4);

        frst::Symbol_Table table;
        auto callable = std::make_shared<CountingCallable>();
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto v1 = evaluate_statement(program[0], table);
        auto v2 = evaluate_statement(program[1], table);
        auto v3 = evaluate_statement(program[2], table);
        auto v4 = evaluate_statement(program[3], table);

        REQUIRE(v1->is<frst::Array>());
        const auto& mapped = v1->raw_get<frst::Array>();
        REQUIRE(mapped.size() == 3);
        CHECK(mapped[0]->get<frst::Int>().value() == 2_f);
        CHECK(mapped[1]->get<frst::Int>().value() == 3_f);
        CHECK(mapped[2]->get<frst::Int>().value() == 4_f);

        REQUIRE(v2->is<frst::Array>());
        const auto& filtered = v2->raw_get<frst::Array>();
        REQUIRE(filtered.size() == 2);
        CHECK(filtered[0]->get<frst::Int>().value() == 2_f);
        CHECK(filtered[1]->get<frst::Int>().value() == 3_f);

        REQUIRE(v3->is<frst::Int>());
        CHECK(v3->get<frst::Int>().value() == 6_f);

        REQUIRE(v4->is<frst::Null>());
        REQUIRE(callable->args.size() == 2);
        CHECK(callable->args[0]->get<frst::Int>().value() == 1_f);
        CHECK(callable->args[1]->get<frst::Int>().value() == 2_f);
    }

    SECTION("Map/filter/reduce/foreach compose with other operators")
    {
        auto result =
            parse("not map [1] with fn (x) -> { x };"
                  "map [1] with fn (x) -> { x + 1 };"
                  "map [1] with f @ g();"
                  "(map [1] with fn (x) -> { x })[0] @ g();"
                  "(filter [1, 2, 3] with fn (x) -> { x > 1 })[0] + 10;"
                  "(reduce [1, 2, 3] with fn (acc, x) -> { acc + x }) * 3");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 6);

        frst::Symbol_Table table;
        auto g_callable = std::make_shared<IdentityCallable>();
        auto f_callable = std::make_shared<IdentityCallable>();
        table.define("g", frst::Value::create(frst::Function{g_callable}));
        table.define("f", frst::Value::create(frst::Function{f_callable}));

        auto v1 = evaluate_statement(program[0], table);
        auto v2 = evaluate_statement(program[1], table);
        auto v3 = evaluate_statement(program[2], table);
        auto v4 = evaluate_statement(program[3], table);
        auto v5 = evaluate_statement(program[4], table);
        auto v6 = evaluate_statement(program[5], table);

        REQUIRE(v1->is<frst::Bool>());
        CHECK(v1->get<frst::Bool>().value() == false);
        REQUIRE(v2->is<frst::Array>());
        CHECK(v2->raw_get<frst::Array>().at(0)->get<frst::Int>().value()
              == 2_f);
        REQUIRE(v3->is<frst::Array>());
        CHECK(v3->raw_get<frst::Array>().at(0)->get<frst::Int>().value()
              == 1_f);
        REQUIRE(v4->is<frst::Int>());
        CHECK(v4->get<frst::Int>().value() == 1_f);
        REQUIRE(v5->is<frst::Int>());
        CHECK(v5->get<frst::Int>().value() == 12_f);
        REQUIRE(v6->is<frst::Int>());
        CHECK(v6->get<frst::Int>().value() == 18_f);
    }

    SECTION("Nested map/filter/reduce/foreach expressions in a program")
    {
        auto result = parse(
            "map (filter [1, 2, 3] with fn (x) -> { x > 1 }) "
            "with fn (x) -> { (reduce [1, 2] with fn (acc, y) -> { acc + y }) "
            "+ x };"
            "reduce (map [1, 2] with fn (x) -> { x }) "
            "with fn (acc, x) -> { acc + x } "
            "init: (reduce [1, 2] with fn (acc, x) -> { acc + x });"
            "map [1] with fn (x) -> { foreach [1] with fn (y) -> { y }; x }");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 3);

        frst::Symbol_Table table;
        auto v1 = evaluate_statement(program[0], table);
        auto v2 = evaluate_statement(program[1], table);
        auto v3 = evaluate_statement(program[2], table);

        REQUIRE(v1->is<frst::Array>());
        const auto& arr = v1->raw_get<frst::Array>();
        REQUIRE(arr.size() == 2);
        CHECK(arr[0]->get<frst::Int>().value() == 5_f);
        CHECK(arr[1]->get<frst::Int>().value() == 6_f);

        REQUIRE(v2->is<frst::Int>());
        CHECK(v2->get<frst::Int>().value() == 6_f);

        REQUIRE(v3->is<frst::Array>());
        const auto& arr2 = v3->raw_get<frst::Array>();
        REQUIRE(arr2.size() == 1);
        CHECK(arr2[0]->get<frst::Int>().value() == 1_f);
    }

    SECTION("Deeply nested higher-order expressions in calls and UFCS")
    {
        auto result =
            parse("def id = fn (x) -> { x };\n"
                  "def inc = fn (x) -> { x + 1 };\n"
                  "id(map [1, 2] with inc)[0];\n"
                  "(filter [1, 2, 3] with fn (x) -> { x > 1 }) @ id();\n"
                  "reduce (filter [1, 2, 3] with fn (x) -> { x > 1 }) "
                  "with fn (acc, x) -> { acc + x } init: 0");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 5);

        frst::Symbol_Table table;
        program[0]->execute(table);
        program[1]->execute(table);

        auto v1 = evaluate_statement(program[2], table);
        auto v2 = evaluate_statement(program[3], table);
        auto v3 = evaluate_statement(program[4], table);

        REQUIRE(v1->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 2_f);

        REQUIRE(v2->is<frst::Array>());
        const auto& arr = v2->raw_get<frst::Array>();
        REQUIRE(arr.size() == 2);
        CHECK(arr[0]->get<frst::Int>().value() == 2_f);
        CHECK(arr[1]->get<frst::Int>().value() == 3_f);

        REQUIRE(v3->is<frst::Int>());
        CHECK(v3->get<frst::Int>().value() == 5_f);
    }

    SECTION("Statement boundaries with map/filter/reduce")
    {
        {
            auto result = parse("map [1] with f def x = 1");
            REQUIRE(result);
            auto program = require_program(result);
            REQUIRE(program.size() == 2);

            frst::Symbol_Table table;
            auto f_callable = std::make_shared<IdentityCallable>();
            table.define("f", frst::Value::create(frst::Function{f_callable}));

            auto v1 = evaluate_statement(program[0], table);
            program[1]->execute(table);

            REQUIRE(v1->is<frst::Array>());
            auto x_val = table.lookup("x");
            REQUIRE(x_val->is<frst::Int>());
            CHECK(x_val->get<frst::Int>().value() == 1_f);
        }

        {
            auto result = parse("def y = map [1] with f z");
            REQUIRE(result);
            auto program = require_program(result);
            REQUIRE(program.size() == 2);

            frst::Symbol_Table table;
            auto f_callable = std::make_shared<IdentityCallable>();
            table.define("f", frst::Value::create(frst::Function{f_callable}));
            auto z_val = frst::Value::create(9_f);
            table.define("z", z_val);

            program[0]->execute(table);
            auto y_val = table.lookup("y");
            REQUIRE(y_val->is<frst::Array>());

            auto v2 = evaluate_statement(program[1], table);
            CHECK(v2 == z_val);
        }
    }

    SECTION("Map/filter/reduce/foreach appear in defs and if expressions")
    {
        auto result = parse("def x = map [1] with fn (v) -> { v };"
                            "def y = filter [1, 2] with fn (v) -> { v > 1 };"
                            "def z = reduce [1, 2] with fn (a, b) -> { a + b };"
                            "if true: map [1] with fn (v) -> { v } else: map "
                            "[2] with fn (v) -> { v };"
                            "foreach [1] with fn (v) -> { v };"
                            "x[0] y[0] z");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 8);

        frst::Symbol_Table table;
        program[0]->execute(table);
        program[1]->execute(table);
        program[2]->execute(table);

        auto if_out = evaluate_statement(program[3], table);
        auto foreach_out = evaluate_statement(program[4], table);
        auto v1 = evaluate_statement(program[5], table);
        auto v2 = evaluate_statement(program[6], table);
        auto v3 = evaluate_statement(program[7], table);

        REQUIRE(if_out->is<frst::Array>());
        REQUIRE(foreach_out->is<frst::Null>());
        REQUIRE(v1->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 1_f);
        REQUIRE(v2->is<frst::Int>());
        CHECK(v2->get<frst::Int>().value() == 2_f);
        REQUIRE(v3->is<frst::Int>());
        CHECK(v3->get<frst::Int>().value() == 3_f);
    }

    SECTION("UFCS allows postfix after newlines")
    {
        auto result = parse("a @ f()\n[0]");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        auto a_val = frst::Value::create(11_f);
        table.define("a", a_val);

        frst::Array arr;
        arr.push_back(a_val);
        auto callable = std::make_shared<Constant_Callable>();
        callable->result = frst::Value::create(std::move(arr));
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto v1 = evaluate_statement(program[0], table);
        CHECK(v1 == a_val);
    }

    SECTION("If expressions can be followed by definitions without separators")
    {
        auto result = parse("if a: b else: c def x = 1");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 2);

        frst::Symbol_Table table;
        auto a_val = frst::Value::create(true);
        auto b_val = frst::Value::create(7_f);
        auto c_val = frst::Value::create(9_f);
        table.define("a", a_val);
        table.define("b", b_val);
        table.define("c", c_val);

        auto v1 = evaluate_statement(program[0], table);
        program[1]->execute(table);
        auto x_val = table.lookup("x");

        CHECK(v1 == b_val);
        REQUIRE(x_val->is<frst::Int>());
        CHECK(x_val->get<frst::Int>().value() == 1_f);
    }

    SECTION("Definitions can follow expressions without separators")
    {
        auto result = parse("1 def x = 2 x");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 3);

        frst::Symbol_Table table;
        auto v1 = evaluate_statement(program[0], table);
        program[1]->execute(table);
        auto v3 = evaluate_statement(program[2], table);

        REQUIRE(v1->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 1_f);
        REQUIRE(v3->is<frst::Int>());
        CHECK(v3->get<frst::Int>().value() == 2_f);
    }

    SECTION("Postfix can cross newlines after lambda expressions")
    {
        auto result = parse("fn () -> { 1 }\n[0]");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 1);
    }

    SECTION("Postfix can cross newlines after map literals")
    {
        auto result = parse("%{a: 1}\n[\"a\"]");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        auto v1 = evaluate_statement(program[0], table);
        REQUIRE(v1->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 1_f);
    }

    SECTION("Lambda calls can be postfixed by indexing")
    {
        auto result = parse("fn (x) -> { [x] }(3)[0]");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        auto v1 = evaluate_statement(program[0], table);
        REQUIRE(v1->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 3_f);
    }

    SECTION("Dot access can return a function which is then called")
    {
        auto result = parse("(%{a: fn () -> { 1 }}).a()");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        auto v1 = evaluate_statement(program[0], table);
        REQUIRE(v1->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 1_f);
    }

    SECTION("Functions stored in array literals can be indexed and called")
    {
        auto result = parse("([fn (x) -> { x }][0])(5)");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        auto v1 = evaluate_statement(program[0], table);
        REQUIRE(v1->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 5_f);
    }

    SECTION(
        "Definitions can use lambdas and if expressions on the right-hand side")
    {
        auto result =
            parse("def make = fn () -> { def y = 1; y }\n"
                  "def choose = if cond: fn () -> { 1 } else: fn () -> { 2 }\n"
                  "make() choose()");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 4);

        frst::Symbol_Table table;
        table.define("cond", frst::Value::create(true));

        program[0]->execute(table);
        program[1]->execute(table);

        auto v1 = evaluate_statement(program[2], table);
        auto v2 = evaluate_statement(program[3], table);

        REQUIRE(v1->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 1_f);
        REQUIRE(v2->is<frst::Int>());
        CHECK(v2->get<frst::Int>().value() == 1_f);
    }

    SECTION("Mixed program combines defs, if, arrays, maps, lambdas, calls, "
            "and indexing")
    {
        auto result = parse("def arr = [1, 2, 3];\n"
                            "def m = %{a: 10, b: 20};\n"
                            "def pick = fn (x) -> { x };\n"
                            "if true: m.a else: m.b;\n"
                            "pick(arr[1]);\n"
                            "m[\"b\"]");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 6);

        frst::Symbol_Table table;
        program[0]->execute(table);
        program[1]->execute(table);
        program[2]->execute(table);

        auto v1 = evaluate_statement(program[3], table);
        auto v2 = evaluate_statement(program[4], table);
        auto v3 = evaluate_statement(program[5], table);

        REQUIRE(v1->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 10_f);
        REQUIRE(v2->is<frst::Int>());
        CHECK(v2->get<frst::Int>().value() == 2_f);
        REQUIRE(v3->is<frst::Int>());
        CHECK(v3->get<frst::Int>().value() == 20_f);
    }

    SECTION("Chained postfix expressions work across maps and calls")
    {
        auto result = parse("obj.f(1)[0].g; arr[0](3)");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 2);

        frst::Symbol_Table table;

        auto f_callable = std::make_shared<Constant_Callable>();
        frst::Map g_map;
        g_map.emplace(frst::Value::create(std::string{"g"}),
                      frst::Value::create(7_f));
        f_callable->result = frst::Value::create(frst::Array{
            frst::Value::create(std::move(g_map)),
        });

        frst::Map obj;
        obj.emplace(frst::Value::create(std::string{"f"}),
                    frst::Value::create(frst::Function{f_callable}));
        table.define("obj", frst::Value::create(std::move(obj)));

        auto arr_callable = std::make_shared<Constant_Callable>();
        arr_callable->result = frst::Value::create(42_f);
        table.define("arr",
                     frst::Value::create(frst::Array{
                         frst::Value::create(frst::Function{arr_callable}),
                     }));

        auto v1 = evaluate_statement(program[0], table);
        auto v2 = evaluate_statement(program[1], table);

        REQUIRE(v1->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 7_f);
        REQUIRE(v2->is<frst::Int>());
        CHECK(v2->get<frst::Int>().value() == 42_f);
    }

    SECTION("Arrays and maps can appear inside larger expressions")
    {
        auto result = parse("([1, 2])[0] + 3; %{a: 1}[\"a\"] == 1");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 2);

        frst::Symbol_Table table;
        auto v1 = evaluate_statement(program[0], table);
        auto v2 = evaluate_statement(program[1], table);

        REQUIRE(v1->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 4_f);
        REQUIRE(v2->is<frst::Bool>());
        CHECK(v2->get<frst::Bool>().value() == true);
    }

    SECTION("Bracketed literals do not swallow following statements")
    {
        auto result = parse("[1] b\n%{a: 1} b");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 4);

        frst::Symbol_Table table;
        table.define("b", frst::Value::create(9_f));

        auto v1 = evaluate_statement(program[0], table);
        auto v2 = evaluate_statement(program[1], table);
        auto v3 = evaluate_statement(program[2], table);
        auto v4 = evaluate_statement(program[3], table);

        REQUIRE(v1->is<frst::Array>());
        CHECK(v2->get<frst::Int>().value() == 9_f);
        REQUIRE(v3->is<frst::Map>());
        CHECK(v4->get<frst::Int>().value() == 9_f);
    }

    SECTION("Whitespace and comments around new constructs")
    {
        auto result = parse("def x = [1,\n# c\n2]\n"
                            "# mid\n"
                            "def y = %{a: 1,\n# c\nb: 2}\n"
                            "def z = fn () -> { ; ; 3 }\n"
                            "x y z()");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 6);

        frst::Symbol_Table table;
        program[0]->execute(table);
        program[1]->execute(table);
        program[2]->execute(table);

        auto v1 = evaluate_statement(program[3], table);
        auto v2 = evaluate_statement(program[4], table);
        auto v3 = evaluate_statement(program[5], table);

        REQUIRE(v1->is<frst::Array>());
        REQUIRE(v2->is<frst::Map>());
        REQUIRE(v3->is<frst::Int>());
        CHECK(v3->get<frst::Int>().value() == 3_f);
    }

    SECTION("Adjacent map literals are separate statements")
    {
        auto result = parse("%{a: 1} %{b: 2}");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 2);
    }

    SECTION("Array literals separated by semicolons are distinct statements")
    {
        auto result = parse("[1, 2]; [1]");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 2);

        frst::Symbol_Table table;
        auto v1 = evaluate_statement(program[0], table);
        auto v2 = evaluate_statement(program[1], table);

        REQUIRE(v1->is<frst::Array>());
        REQUIRE(v2->is<frst::Array>());
        CHECK(v1->raw_get<frst::Array>().size() == 2);
        CHECK(v2->raw_get<frst::Array>().size() == 1);
    }

    SECTION("Array literals as adjacent statements")
    {
        auto result = parse("[1][0] 2");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 2);

        frst::Symbol_Table table;
        auto v1 = evaluate_statement(program[0], table);
        auto v2 = evaluate_statement(program[1], table);

        REQUIRE(v1->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 1_f);
        REQUIRE(v2->is<frst::Int>());
        CHECK(v2->get<frst::Int>().value() == 2_f);
    }

    SECTION("Array literals followed by indexing across newlines")
    {
        auto result = parse("[]\n[1]");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 1);
    }

    SECTION("Parenthesized expressions are valid statements")
    {
        auto result = parse("((2))");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        auto value = evaluate_statement(program[0], table);
        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 2_f);
    }

    SECTION("Mixed grammar in a single program")
    {
        auto result = parse("1+2*3; not false; (1<2)==true; false or true; -5");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 5);

        frst::Symbol_Table table;
        auto v1 = evaluate_statement(program[0], table);
        auto v2 = evaluate_statement(program[1], table);
        auto v3 = evaluate_statement(program[2], table);
        auto v4 = evaluate_statement(program[3], table);
        auto v5 = evaluate_statement(program[4], table);

        REQUIRE(v1->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 7_f);
        REQUIRE(v2->is<frst::Bool>());
        CHECK(v2->get<frst::Bool>().value() == true);
        REQUIRE(v3->is<frst::Bool>());
        CHECK(v3->get<frst::Bool>().value() == true);
        REQUIRE(v4->is<frst::Bool>());
        CHECK(v4->get<frst::Bool>().value() == true);
        REQUIRE(v5->is<frst::Int>());
        CHECK(v5->get<frst::Int>().value() == -5_f);
    }

    SECTION("Postfix expressions in a program")
    {
        auto result = parse("arr[0]; f(1,2); obj.key; arr[\n0\n]; obj.f(3)");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 5);

        frst::Symbol_Table table;

        auto arr_val = frst::Value::create(frst::Array{
            frst::Value::create(1_f),
            frst::Value::create(2_f),
        });
        table.define("arr", arr_val);

        auto f_callable = std::make_shared<Constant_Callable>();
        f_callable->result = frst::Value::create(9_f);
        table.define("f", frst::Value::create(frst::Function{f_callable}));

        auto obj_callable = std::make_shared<Constant_Callable>();
        obj_callable->result = frst::Value::create(11_f);

        frst::Map obj;
        obj.emplace(frst::Value::create(std::string{"key"}),
                    frst::Value::create(5_f));
        obj.emplace(frst::Value::create(std::string{"f"}),
                    frst::Value::create(frst::Function{obj_callable}));
        table.define("obj", frst::Value::create(std::move(obj)));

        auto v1 = evaluate_statement(program[0], table);
        auto v2 = evaluate_statement(program[1], table);
        auto v3 = evaluate_statement(program[2], table);
        auto v4 = evaluate_statement(program[3], table);
        auto v5 = evaluate_statement(program[4], table);

        REQUIRE(v1->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 1_f);
        REQUIRE(v2->is<frst::Int>());
        CHECK(v2->get<frst::Int>().value() == 9_f);
        REQUIRE(v3->is<frst::Int>());
        CHECK(v3->get<frst::Int>().value() == 5_f);
        REQUIRE(v4->is<frst::Int>());
        CHECK(v4->get<frst::Int>().value() == 1_f);
        REQUIRE(v5->is<frst::Int>());
        CHECK(v5->get<frst::Int>().value() == 11_f);
    }

    SECTION("If expressions in a program")
    {
        auto result = parse("if true: 1 else: 2;\n"
                            "if false: 1;\n"
                            "if false: 1 elif true: 3 else: 4;\n"
                            "if false: 1 elif false: 2;\n");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 4);

        frst::Symbol_Table table;

        auto v1 = evaluate_statement(program[0], table);
        auto v2 = evaluate_statement(program[1], table);
        auto v3 = evaluate_statement(program[2], table);
        auto v4 = evaluate_statement(program[3], table);

        REQUIRE(v1->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 1_f);
        REQUIRE(v2->is<frst::Null>());
        REQUIRE(v3->is<frst::Int>());
        CHECK(v3->get<frst::Int>().value() == 3_f);
        REQUIRE(v4->is<frst::Null>());
    }

    SECTION("Definition statements in a program")
    {
        auto result = parse("def x = 1;\n"
                            "def y = x + 2;\n"
                            "y\n");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 3);

        frst::Symbol_Table table;
        program[0]->execute(table);
        program[1]->execute(table);
        auto value = evaluate_statement(program[2], table);

        REQUIRE(value->is<frst::Int>());
        CHECK(value->get<frst::Int>().value() == 3_f);
    }

    SECTION("Definitions separated by whitespace and semicolons")
    {
        auto result = parse("def a = 1;; def b = 2; a; b");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 4);

        frst::Symbol_Table table;
        program[0]->execute(table);
        program[1]->execute(table);
        auto v1 = evaluate_statement(program[2], table);
        auto v2 = evaluate_statement(program[3], table);

        REQUIRE(v1->is<frst::Int>());
        REQUIRE(v2->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 1_f);
        CHECK(v2->get<frst::Int>().value() == 2_f);
    }

    SECTION("If expressions with comments and whitespace in a program")
    {
        auto result = parse("if true : 1 else : 2\n"
                            "if true: # c\n"
                            "1 else: 2\n"
                            "if false:\n"
                            "1\n"
                            "else:\n"
                            "2\n");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 3);

        frst::Symbol_Table table;

        auto v1 = evaluate_statement(program[0], table);
        auto v2 = evaluate_statement(program[1], table);
        auto v3 = evaluate_statement(program[2], table);

        REQUIRE(v1->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 1_f);
        REQUIRE(v2->is<frst::Int>());
        CHECK(v2->get<frst::Int>().value() == 1_f);
        REQUIRE(v3->is<frst::Int>());
        CHECK(v3->get<frst::Int>().value() == 2_f);
    }

    SECTION("Statements after an if expression are separate statements")
    {
        auto result = parse("if a: b\nelse: c\nfun()");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 2);

        frst::Symbol_Table table;

        auto a_val = frst::Value::create(true);
        auto b_val = frst::Value::create(3_f);
        auto c_val = frst::Value::create(4_f);

        table.define("a", a_val);
        table.define("b", b_val);
        table.define("c", c_val);

        struct RecordingCallable final : frst::Callable
        {
            mutable int calls = 0;
            frst::Value_Ptr result;

            frst::Value_Ptr call(
                std::span<const frst::Value_Ptr>) const override
            {
                ++calls;
                return result ? result : frst::Value::null();
            }

            std::string debug_dump() const override
            {
                return "<recording>";
            }
        };

        auto callable = std::make_shared<RecordingCallable>();
        callable->result = frst::Value::create(9_f);
        table.define("fun", frst::Value::create(frst::Function{callable}));

        auto v1 = evaluate_statement(program[0], table);
        auto v2 = evaluate_statement(program[1], table);

        CHECK(v1 == b_val);
        CHECK(v2 == callable->result);
        CHECK(callable->calls == 1);
    }

    SECTION("Pathological postfix whitespace and comments in a program")
    {
        auto result = parse("arr[ # c\n 1 ]\n"
                            "obj .\n key\n"
                            "f( # c\n )\n"
                            "arr[\n -1\n]\n"
                            "obj.# c\nkey\n"
                            "obj .\n inner .\n value\n");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 6);

        frst::Symbol_Table table;

        auto arr_val = frst::Value::create(frst::Array{
            frst::Value::create(1_f),
            frst::Value::create(2_f),
            frst::Value::create(3_f),
        });
        table.define("arr", arr_val);

        auto f_callable = std::make_shared<Constant_Callable>();
        f_callable->result = frst::Value::create(42_f);
        table.define("f", frst::Value::create(frst::Function{f_callable}));

        frst::Map inner;
        inner.emplace(frst::Value::create(std::string{"value"}),
                      frst::Value::create(9_f));

        frst::Map obj;
        obj.emplace(frst::Value::create(std::string{"key"}),
                    frst::Value::create(7_f));
        obj.emplace(frst::Value::create(std::string{"inner"}),
                    frst::Value::create(std::move(inner)));
        table.define("obj", frst::Value::create(std::move(obj)));

        auto v1 = evaluate_statement(program[0], table);
        auto v2 = evaluate_statement(program[1], table);
        auto v3 = evaluate_statement(program[2], table);
        auto v4 = evaluate_statement(program[3], table);
        auto v5 = evaluate_statement(program[4], table);
        auto v6 = evaluate_statement(program[5], table);

        REQUIRE(v1->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 2_f);
        REQUIRE(v2->is<frst::Int>());
        CHECK(v2->get<frst::Int>().value() == 7_f);
        REQUIRE(v3->is<frst::Int>());
        CHECK(v3->get<frst::Int>().value() == 42_f);
        REQUIRE(v4->is<frst::Int>());
        CHECK(v4->get<frst::Int>().value() == 3_f);
        REQUIRE(v5->is<frst::Int>());
        CHECK(v5->get<frst::Int>().value() == 7_f);
        REQUIRE(v6->is<frst::Int>());
        CHECK(v6->get<frst::Int>().value() == 9_f);
    }

    SECTION("Whitespace and comments are ignored between statements")
    {
        auto result = parse("# comment\n42;\n# next\n\"hi\"");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 2);

        frst::Symbol_Table table;
        auto v1 = evaluate_statement(program[0], table);
        auto v2 = evaluate_statement(program[1], table);

        REQUIRE(v1->is<frst::Int>());
        CHECK(v1->get<frst::Int>().value() == 42_f);
        REQUIRE(v2->is<frst::String>());
        CHECK(v2->get<frst::String>().value() == "hi");
    }

    SECTION("Invalid tokens after a statement cause parse errors")
    {
        const std::string_view cases[] = {"1)", "(1"};

        for (const auto& input : cases)
        {
            auto result = parse(input);
            CHECK_FALSE(result);
            CHECK(result.error_count() >= 1);
        }
    }

    SECTION("Invalid input rejects the entire program")
    {
        const std::string_view cases[] = {
            ")",
            "(",
            ";; )",
            "[1;2]",
            "%{a: 1; b: 2}",
            "%{a: 1\nb: 2}",
            "if true: def x = 1 else: 2",
            "def x = [1;2]",
            "def x = %{a: 1; b: 2}",
            "fn (x, ) -> {}",
            "obj.if",
            "def if = 1",
            "a @ f",
            "a @ obj.key",
        };

        for (const auto& input : cases)
        {
            auto result = parse(input);
            CHECK_FALSE(result);
            CHECK(result.error_count() >= 1);
        }
    }
}
