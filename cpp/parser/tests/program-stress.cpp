#include <catch2/catch_test_macros.hpp>

#include <frost/symbol-table.hpp>
#include <frost/testing/stringmaker-specializations.hpp>
#include <frost/value.hpp>

#include <lexy/action/parse.hpp>
#include <lexy/callback.hpp>
#include <lexy/input/string_input.hpp>

#include <ranges>
#include <sstream>

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

frst::Value_Ptr run_statement(const frst::ast::Statement::Ptr& statement,
                              frst::Symbol_Table& table)
{
    if (auto* expr =
            dynamic_cast<const frst::ast::Expression*>(statement.get()))
    {
        return expr->evaluate(table);
    }
    statement->execute(table);
    return frst::Value::null();
}

std::string dump_statement(const frst::ast::Statement::Ptr& statement)
{
    std::ostringstream out;
    statement->debug_dump_ast(out);
    return out.str();
}
} // namespace

TEST_CASE("Parser Program Stress Tests")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        auto src = lexy::string_input<lexy::utf8_encoding>(input);
        return lexy::parse<Program_Root>(src, lexy::noop);
    };

    auto require_equivalent_programs = [&](std::string_view left_src,
                                           std::string_view right_src) {
        auto left_result = parse(left_src);
        auto right_result = parse(right_src);
        REQUIRE(left_result);
        REQUIRE(right_result);

        auto left_program = require_program(left_result);
        auto right_program = require_program(right_result);
        REQUIRE(left_program.size() == right_program.size());

        for (auto&& [lhs, rhs] : std::views::zip(left_program, right_program))
        {
            CHECK(dump_statement(lhs) == dump_statement(rhs));
        }
    };

    SECTION("Large mixed program parses into expected statement count")
    {
        auto result = parse(R"FROST(
# definitions
def inc = fn (x) -> { x + 1 };
def dec = fn (x) -> { x - 1 };
def id = fn (x) -> { x };
def arr = [1, 2, 3, 4,];
def nested = [[1], [2, 3],];
def m = {a: 1, b: 2, [3]: 4,};
def m2 = {foo: {bar: 1}, [0]: [1, 2],};
def f = fn () -> { 1 };
def g = fn (x) -> { [x] };

if true: arr else: nested;
if false: 1 elif true: 2 else: 3;

map arr with inc;
filter arr with fn (x) -> { x > 2 };
reduce arr with fn (acc, x) -> { acc + x };
reduce arr with fn (acc, x) -> { acc + x } init: 0;
foreach arr with fn (x) -> { x };

map m with fn (k, v) -> { {[k]: v} };
filter m with fn (k, v) -> { v };
reduce m with fn (acc, k, v) -> { acc } init: 0;

arr[0];
m.a;
m["b"];
(m).a;
(m)["b"];
1 @ g();
(1 @ g())[0];
id(map arr with inc)[0];
(map arr with inc)[0];
not fn () -> { null };

1; 2; 3;
def x = 1; def y = 2;
x; y;
)FROST");
        REQUIRE(result);
        auto program = require_program(result);
        CHECK(program.size() == 36);
    }

    SECTION("Large program evaluates consistently")
    {
        auto result = parse(R"FROST(
def inc = fn (x) -> { x + 1 };
def id = fn (x) -> { x };
def wrap = fn (x) -> { [x] };
def pick = fn (x) -> { x };
def arr = [1, 2, 3, 4];
def mapped = map arr with inc;
def filtered = filter mapped with fn (x) -> { x > 2 };
def reduced = reduce filtered with fn (acc, x) -> { acc + x } init: 0;
def m = {a: 1, b: 2};
def m2 = map m with fn (k, v) -> { {[k]: v + 1} };
def m3 = filter m2 with fn (k, v) -> { v > 2 };
foreach m3 with fn (k, v) -> { v };
def uf = 1 @ wrap();
def idx = (1 @ wrap())[0];
def res = pick(reduced) + idx;
res
)FROST");
        REQUIRE(result);
        auto program = require_program(result);
        REQUIRE(program.size() == 16);

        frst::Symbol_Table table;
        frst::Value_Ptr last = frst::Value::null();
        for (const auto& stmt : program)
        {
            last = run_statement(stmt, table);
        }

        REQUIRE(last->is<frst::Int>());
        CHECK(last->get<frst::Int>().value() == 13_f);
    }

    SECTION("Threaded call matches function call in debug dumps")
    {
        require_equivalent_programs("a @ b()", "b(a)");
    }

    SECTION("Chained threaded calls match nested call structure in debug dumps")
    {
        require_equivalent_programs("a @ b() @ c()", "c(b(a))");
    }

    SECTION("Dot access matches index access in debug dumps")
    {
        require_equivalent_programs("obj.key", "obj[\"key\"]");
    }

    SECTION("Redundant parentheses do not alter AST dumps in large programs")
    {
        require_equivalent_programs(
            R"FROST(
def inc = fn (x) -> { x + 1 };
def arr = [1, 2, 3];
def m = {a: 1, b: 2};
map arr with inc;
filter arr with fn (x) -> { x > 1 };
reduce arr with fn (acc, x) -> { acc + x } init: 0;
(arr[0] + m.a) * 2;
a @ f(1, 2);
id(map arr with inc)[0];
)FROST",
            R"FROST(
def inc = fn (x) -> { ((x) + (1)) };
def arr = ( [ (1), (2), (3), ] );
def m = ( {a: (1), b: (2),} );
(map (arr) with (inc));
(filter (arr) with fn (x) -> { ((x) > (1)) });
(reduce (arr) with fn (acc, x) -> { (acc) + (x) } init: (0));
(((arr)[0]) + ((m).a)) * (2);
((a)) @ f((1), (2));
id((map (arr) with (inc)))[0];
)FROST");
    }

    SECTION("Parentheses around primary expressions do not change AST dumps")
    {
        require_equivalent_programs(
            "fn () -> { 1 }(); [1, 2][0]; {a: 1}.a; not false",
            "(fn () -> { 1 })(); ([1, 2])[0]; ({a: 1}).a; not (false)");
    }
}
