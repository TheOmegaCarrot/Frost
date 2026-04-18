#include <catch2/catch_test_macros.hpp>

#include <frost/ast.hpp>
#include <frost/closure.hpp>
#include <frost/parser.hpp>
#include <frost/symbol-table.hpp>
#include <frost/testing/stringmaker-specializations.hpp>
#include <frost/value.hpp>

using namespace frst::literals;
using namespace std::literals;

namespace
{
auto parse(std::string_view input)
{
    return frst::parse_program(std::string{input});
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
        REQUIRE(result.has_value());
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
        REQUIRE(result.has_value());
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
        REQUIRE(result.has_value());
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
        REQUIRE(result.has_value());
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
        REQUIRE(result.has_value());
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
        REQUIRE(result.has_value());
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
        REQUIRE(result.has_value());
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        frst::Execution_Context ctx{.symbols = table};
        program[0]->execute(ctx);

        auto fn = table.lookup("f")->get<frst::Function>().value();
        auto closure = std::dynamic_pointer_cast<const frst::Closure>(fn);
        REQUIRE(closure);
        CHECK_FALSE(closure->debug_capture_table().has("f"));
    }

    SECTION("defn inside a lambda body")
    {
        auto result = parse("(fn() -> {\n"
                            "    defn double(x) -> x * 2\n"
                            "    double(5)\n"
                            "})()");
        REQUIRE(result.has_value());
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
        REQUIRE(result.has_value());
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        frst::Symbol_Table table;
        frst::Execution_Context ctx{.symbols = table};
        program[0]->execute(ctx);

        auto value = table.lookup("f");
        REQUIRE(value->is<frst::Function>());

        auto out = call_function(value, {frst::Value::create(6_f)});
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 12_f);
    }

    SECTION("export defn enables recursion via self-name")
    {
        auto result =
            parse("export defn fact(n) -> if n <= 1: 1 else: n * fact(n - 1)");
        REQUIRE(result.has_value());
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

    SECTION("export defn is rejected inside a lambda body")
    {
        auto result = parse("fn() -> { export defn f(x) -> x }");
        CHECK(not result);
    }

    SECTION("defn without parens is rejected")
    {
        CHECK(not parse("defn f -> 1"));
    }

    SECTION("defn with missing arrow is rejected")
    {
        CHECK(not parse("defn f(x) { x }"));
    }

    SECTION("defn with empty body is rejected")
    {
        CHECK(not parse("defn f(x) ->"));
    }

    SECTION("multiple defn statements parse as separate definitions")
    {
        auto result = parse("defn f(x) -> x\ndefn g(x) -> x * 2");
        REQUIRE(result.has_value());
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
        REQUIRE(result.has_value());
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
        REQUIRE(result.has_value());
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
        REQUIRE(result.has_value());
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
        REQUIRE(result.has_value());
        auto program = std::move(result).value();
        REQUIRE(program.size() == 2);

        frst::Symbol_Table table;
        frst::Execution_Context ctx{.symbols = table};
        program[0]->execute(ctx);
        program[1]->execute(ctx);

        auto fn = table.lookup("f")->get<frst::Function>().value();
        auto closure = std::dynamic_pointer_cast<const frst::Closure>(fn);
        REQUIRE(closure);
        CHECK(closure->debug_capture_table().has("base"));
        CHECK_FALSE(closure->debug_capture_table().has("f"));
        CHECK_FALSE(closure->debug_capture_table().has("x"));
    }

    SECTION("defn inside a do block")
    {
        auto result = parse("do {\n"
                            "    defn double(x) -> x * 2\n"
                            "    double(6)\n"
                            "}");
        REQUIRE(result.has_value());
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
        REQUIRE(result.has_value());
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
        REQUIRE(result.has_value());
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
        CHECK(not parse("def defn = 1"));
        CHECK(not parse("def x = defn"));
    }

    SECTION("duplicate parameters are rejected")
    {
        CHECK(not parse("defn f(x, x) -> x"));
    }

    SECTION("keyword as parameter is rejected")
    {
        CHECK(not parse("defn f(if) -> if"));
        CHECK(not parse("defn f(fn) -> fn"));
    }

    SECTION("defn Lambda gets source range from name to end of body")
    {
        // "defn f(x) -> x + 1"
        //       ^           ^
        //  col 6            col 18
        // The Lambda spans from the name 'f' to end of body 'x + 1'
        auto result = parse("defn f(x) -> x + 1");
        REQUIRE(result.has_value());
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        auto nodes = program[0]->walk() | std::ranges::to<std::vector>();
        // Define -> Lambda -> ...
        REQUIRE(nodes.size() >= 2);
        auto lambda_range = nodes[1]->source_range();
        CHECK(lambda_range.begin.line == 1);
        CHECK(lambda_range.begin.column == 6);
        CHECK(lambda_range.end.line == 1);
        CHECK(lambda_range.end.column == 18);
    }

    SECTION("export defn Lambda gets source range")
    {
        // "export defn g(x) -> x * 2"
        //              ^           ^
        //         col 13           col 25
        auto result = parse("export defn g(x) -> x * 2");
        REQUIRE(result.has_value());
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        auto nodes = program[0]->walk() | std::ranges::to<std::vector>();
        REQUIRE(nodes.size() >= 2);
        auto lambda_range = nodes[1]->source_range();
        CHECK(lambda_range.begin.line == 1);
        CHECK(lambda_range.begin.column == 13);
        CHECK(lambda_range.end.line == 1);
        CHECK(lambda_range.end.column == 25);
    }

    SECTION("defn Lambda range excludes trailing whitespace")
    {
        // "defn f(x) -> x   " — trailing spaces should not inflate end
        auto result = parse("defn f(x) -> x   ");
        REQUIRE(result.has_value());
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        auto nodes = program[0]->walk() | std::ranges::to<std::vector>();
        REQUIRE(nodes.size() >= 2);
        auto lambda_range = nodes[1]->source_range();
        CHECK(lambda_range.begin.column == 6);
        CHECK(lambda_range.end.column == 14);
    }

    SECTION("defn binding gets source range of the identifier")
    {
        // "defn square(x) -> x * x"
        //       ^~~~~~
        //  col 6     col 11
        auto result = parse("defn square(x) -> x * x");
        REQUIRE(result.has_value());
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        auto nodes = program[0]->walk() | std::ranges::to<std::vector>();
        auto* binding =
            dynamic_cast<const frst::ast::Destructure_Binding*>(nodes.back());
        REQUIRE(binding);
        auto range = binding->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 6);
        CHECK(range.end.line == 1);
        CHECK(range.end.column == 11);
    }

    SECTION("export defn binding gets source range of the identifier")
    {
        // "export defn greet(name) -> name"
        //              ^~~~~
        //         col 13   col 17
        auto result = parse("export defn greet(name) -> name");
        REQUIRE(result.has_value());
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        auto nodes = program[0]->walk() | std::ranges::to<std::vector>();
        auto* binding =
            dynamic_cast<const frst::ast::Destructure_Binding*>(nodes.back());
        REQUIRE(binding);
        auto range = binding->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 13);
        CHECK(range.end.line == 1);
        CHECK(range.end.column == 17);
    }

    SECTION("defn binding range is not no_range")
    {
        auto result = parse("defn f(x) -> x");
        REQUIRE(result.has_value());
        auto program = std::move(result).value();
        REQUIRE(program.size() == 1);

        auto nodes = program[0]->walk() | std::ranges::to<std::vector>();
        auto* binding =
            dynamic_cast<const frst::ast::Destructure_Binding*>(nodes.back());
        REQUIRE(binding);
        auto range = binding->source_range();
        // no_range is {0,0}-{0,0}; a valid range must have nonzero positions
        CHECK(range.begin.line > 0);
        CHECK(range.begin.column > 0);
    }
}
