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

    frst::Value_Ptr call(const std::vector<frst::Value_Ptr>&) const override
    {
        return result ? result : frst::Value::null();
    }

    std::string debug_dump() const override
    {
        return "<constant>";
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
        auto result = parse("1; \"s\"; true; null");
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
        REQUIRE(v2->is<frst::String>());
        CHECK(v2->get<frst::String>().value() == "s");
        REQUIRE(v3->is<frst::Bool>());
        CHECK(v3->get<frst::Bool>().value() == true);
        REQUIRE(v4->is<frst::Null>());
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
        auto result =
            parse("1+2*3; not false; (1<2)==true; false or true; -5");
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
        auto result = parse(
            "if true: 1 else: 2;\n"
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
        auto result = parse(
            "def x = 1;\n"
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
        auto result = parse(
            "if true : 1 else : 2\n"
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

            frst::Value_Ptr call(const std::vector<frst::Value_Ptr>&) const override
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
        auto result = parse(
            "arr[ # c\n 1 ]\n"
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
        };

        for (const auto& input : cases)
        {
            auto result = parse(input);
            CHECK_FALSE(result);
            CHECK(result.error_count() >= 1);
        }
    }
}
