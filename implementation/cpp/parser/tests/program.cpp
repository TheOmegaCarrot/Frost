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
