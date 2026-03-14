#include <catch2/catch_test_macros.hpp>

#include <frost/ast.hpp>
#include <frost/closure.hpp>
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

auto parse(std::string_view input)
{
    auto src = lexy::string_input<lexy::utf8_encoding>(input);
    return lexy::parse<Program_Root>(src, lexy::noop);
}

frst::Value_Ptr call_function(const frst::Value_Ptr& value,
                              std::vector<frst::Value_Ptr> args)
{
    REQUIRE(value->is<frst::Function>());
    auto fn = value->get<frst::Function>().value();
    return fn->call(args);
}
} // namespace

TEST_CASE("Parser Defn Statements")
{
    SECTION("defn binds name to a function")
    {
        auto result = parse("defn f(x) -> x");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        frst::Execution_Context ctx{.symbols = table};
        program[0]->execute(ctx);

        auto value = table.lookup("f");
        REQUIRE(value->is<frst::Function>());
        auto out = call_function(value, {frst::Value::create(7_f)});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 7_f);
    }

    SECTION("defn with multiple params")
    {
        auto result = parse("defn add(x, y) -> x + y");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        frst::Execution_Context ctx{.symbols = table};
        program[0]->execute(ctx);

        auto out =
            call_function(table.lookup("add"),
                          {frst::Value::create(3_f), frst::Value::create(4_f)});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 7_f);
    }

    SECTION("defn with no params")
    {
        auto result = parse("defn f() -> 42");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        frst::Execution_Context ctx{.symbols = table};
        program[0]->execute(ctx);

        auto out = call_function(table.lookup("f"), {});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 42_f);
    }

    SECTION("defn with vararg-only")
    {
        auto result = parse("defn f(...args) -> args");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        frst::Execution_Context ctx{.symbols = table};
        program[0]->execute(ctx);

        auto out = call_function(table.lookup("f"), {frst::Value::create(1_f),
                                                     frst::Value::create(2_f)});
        REQUIRE(out->is<frst::Array>());
        CHECK(out->raw_get<frst::Array>().size() == 2);
    }

    SECTION("defn with params and vararg")
    {
        auto result = parse("defn f(x, ...rest) -> rest");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        frst::Execution_Context ctx{.symbols = table};
        program[0]->execute(ctx);

        auto out = call_function(table.lookup("f"), {frst::Value::create(1_f),
                                                     frst::Value::create(2_f),
                                                     frst::Value::create(3_f)});
        REQUIRE(out->is<frst::Array>());
        CHECK(out->raw_get<frst::Array>().size() == 2);
    }

    SECTION("defn self-name enables recursion")
    {
        auto result =
            parse("defn fact(n) -> if n <= 1: 1 else: n * fact(n - 1)");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        frst::Execution_Context ctx{.symbols = table};
        program[0]->execute(ctx);

        auto out =
            call_function(table.lookup("fact"), {frst::Value::create(5_f)});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 120_f);
    }

    SECTION("defn self-name is the function name")
    {
        auto result = parse("defn f(x) -> x");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        frst::Execution_Context ctx{.symbols = table};
        program[0]->execute(ctx);

        auto fn = table.lookup("f")->get<frst::Function>().value();
        auto closure = std::dynamic_pointer_cast<frst::Closure>(fn);
        REQUIRE(closure);
        CHECK(closure->debug_capture_table().has("f"));
    }

    SECTION("defn inside a lambda body")
    {
        auto result = parse("(fn() -> {\n"
                            "    defn double(x) -> x * 2\n"
                            "    double(5)\n"
                            "})()");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto* expr =
            dynamic_cast<const frst::ast::Expression*>(program[0].get());
        REQUIRE(expr);
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 10_f);
    }

    SECTION("export defn binds and exports the name")
    {
        auto result = parse("export defn f(x) -> x * 2");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        frst::Execution_Context ctx{.symbols = table};
        auto exports = program[0]->execute(ctx);
        REQUIRE(exports.has_value());
        REQUIRE(exports->size() == 1);

        auto it = exports->find(frst::Value::create("f"s));
        REQUIRE(it != exports->end());
        REQUIRE(it->second->is<frst::Function>());

        auto out = call_function(it->second, {frst::Value::create(6_f)});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 12_f);
    }

    SECTION("export defn enables recursion via self-name")
    {
        auto result =
            parse("export defn fact(n) -> if n <= 1: 1 else: n * fact(n - 1)");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        frst::Execution_Context ctx{.symbols = table};
        auto exports = program[0]->execute(ctx);
        REQUIRE(exports.has_value());

        auto it = exports->find(frst::Value::create("fact"s));
        REQUIRE(it != exports->end());
        auto out = call_function(it->second, {frst::Value::create(5_f)});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 120_f);
    }

    SECTION("export defn is rejected inside a lambda body")
    {
        auto result = parse("fn() -> { export defn f(x) -> x }");
        CHECK_FALSE(result);
    }

    SECTION("defn without parens is rejected")
    {
        CHECK_FALSE(parse("defn f -> 1"));
    }

    SECTION("defn with missing arrow is rejected")
    {
        CHECK_FALSE(parse("defn f(x) { x }"));
    }

    SECTION("defn with empty body is rejected")
    {
        CHECK_FALSE(parse("defn f(x) ->"));
    }

    SECTION("multiple defn statements parse as separate definitions")
    {
        auto result = parse("defn f(x) -> x\ndefn g(x) -> x * 2");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 2);

        frst::Symbol_Table table;
        frst::Execution_Context ctx{.symbols = table};
        program[0]->execute(ctx);
        program[1]->execute(ctx);

        auto f_out =
            call_function(table.lookup("f"), {frst::Value::create(3_f)});
        auto g_out =
            call_function(table.lookup("g"), {frst::Value::create(3_f)});
        CHECK(f_out->get<frst::Int>().value() == 3_f);
        CHECK(g_out->get<frst::Int>().value() == 6_f);
    }

    SECTION("defn with block body")
    {
        auto result = parse("defn f(x) -> {\n"
                            "    def y = x + 1\n"
                            "    y * 2\n"
                            "}");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        frst::Execution_Context ctx{.symbols = table};
        program[0]->execute(ctx);

        auto out = call_function(table.lookup("f"), {frst::Value::create(4_f)});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 10_f);
    }

    SECTION("defn with block body enables recursion")
    {
        auto result = parse("defn sum(n) -> {\n"
                            "    if n <= 0: 0\n"
                            "    else: n + sum(n - 1)\n"
                            "}");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        frst::Execution_Context ctx{.symbols = table};
        program[0]->execute(ctx);

        auto out =
            call_function(table.lookup("sum"), {frst::Value::create(4_f)});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 10_f);
    }

    SECTION("defn captures outer variables")
    {
        auto result = parse("def base = 10\ndefn f(x) -> x + base");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 2);

        frst::Symbol_Table table;
        frst::Execution_Context ctx{.symbols = table};
        program[0]->execute(ctx);
        program[1]->execute(ctx);

        auto out = call_function(table.lookup("f"), {frst::Value::create(5_f)});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 15_f);
    }

    SECTION("defn captures outer variable but not its own name")
    {
        auto result = parse("def base = 10\ndefn f(x) -> x + base");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 2);

        frst::Symbol_Table table;
        frst::Execution_Context ctx{.symbols = table};
        program[0]->execute(ctx);
        program[1]->execute(ctx);

        auto fn = table.lookup("f")->get<frst::Function>().value();
        auto closure = std::dynamic_pointer_cast<frst::Closure>(fn);
        REQUIRE(closure);
        CHECK(closure->debug_capture_table().has("base"));
        CHECK(closure->debug_capture_table().has("f"));
        CHECK_FALSE(closure->debug_capture_table().has("x"));
    }

    SECTION("defn inside a do block")
    {
        auto result = parse("do {\n"
                            "    defn double(x) -> x * 2\n"
                            "    double(6)\n"
                            "}");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto* expr =
            dynamic_cast<const frst::ast::Expression*>(program[0].get());
        REQUIRE(expr);
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 12_f);
    }

    SECTION("space between name and paren is accepted")
    {
        auto result = parse("defn f (x) -> x");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        frst::Execution_Context ctx{.symbols = table};
        program[0]->execute(ctx);

        auto out = call_function(table.lookup("f"), {frst::Value::create(9_f)});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 9_f);
    }

    SECTION("body starting on a new line is accepted")
    {
        auto result = parse("defn f(x) ->\n    x + 1");
        REQUIRE(result);
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        frst::Execution_Context ctx{.symbols = table};
        program[0]->execute(ctx);

        auto out = call_function(table.lookup("f"), {frst::Value::create(3_f)});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 4_f);
    }

    SECTION("defn is a reserved keyword and cannot be used as an identifier")
    {
        CHECK_FALSE(parse("def defn = 1"));
        CHECK_FALSE(parse("def x = defn"));
    }

    SECTION("duplicate parameters are rejected")
    {
        CHECK_THROWS(parse("defn f(x, x) -> x"));
    }

    SECTION("keyword as parameter is rejected")
    {
        CHECK_FALSE(parse("defn f(if) -> if"));
        CHECK_FALSE(parse("defn f(fn) -> fn"));
    }
}
