#include <catch2/catch_test_macros.hpp>

#include <frost/ast.hpp>
#include <frost/parser.hpp>
#include <frost/symbol-table.hpp>
#include <frost/testing/stringmaker-specializations.hpp>
#include <frost/value.hpp>

using namespace frst::literals;

namespace
{
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
    std::string name() const override
    {
        return debug_dump();
    }
};

frst::Value_Ptr evaluate_expression(const frst::ast::Statement::Ptr& statement,
                                    frst::Symbol_Table& table)
{
    auto* expr = dynamic_cast<const frst::ast::Expression*>(statement.get());
    REQUIRE(expr);
    frst::Evaluation_Context ctx{.symbols = table};
    return expr->evaluate(ctx);
}
} // namespace

TEST_CASE("Parser Array Literals")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        return frst::parse_data(std::string{input});
    };

    SECTION("Empty array literal is valid")
    {
        auto result = parse("[]");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Array>());
        CHECK(out->raw_get<frst::Array>().empty());
    }

    SECTION("Array elements preserve order")
    {
        auto result = parse("[1, 2, 3]");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 3);
        CHECK(arr[0]->get<frst::Int>().value() == 1_f);
        CHECK(arr[1]->get<frst::Int>().value() == 2_f);
        CHECK(arr[2]->get<frst::Int>().value() == 3_f);
    }

    SECTION("Trailing commas are accepted")
    {
        auto result = parse("[1, 2,]");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 2);
        CHECK(arr[0]->get<frst::Int>().value() == 1_f);
        CHECK(arr[1]->get<frst::Int>().value() == 2_f);
    }

    SECTION("Commas are required between elements")
    {
        CHECK(not parse("[1 2]"));
    }

    SECTION("Whitespace and comments are allowed between elements")
    {
        auto result = parse("[ 1 , # comment\n 2 ,\n 3 ]");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 3);
        CHECK(arr[0]->get<frst::Int>().value() == 1_f);
        CHECK(arr[1]->get<frst::Int>().value() == 2_f);
        CHECK(arr[2]->get<frst::Int>().value() == 3_f);
    }

    SECTION("Nested arrays parse and evaluate")
    {
        auto result = parse("[[1], [2, 3]]");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 2);
        REQUIRE(arr[0]->is<frst::Array>());
        REQUIRE(arr[1]->is<frst::Array>());
        CHECK(arr[0]->raw_get<frst::Array>().size() == 1);
        CHECK(arr[1]->raw_get<frst::Array>().size() == 2);
    }

    SECTION("Array elements can be complex expressions")
    {
        auto result = parse("[1+2, if true: 3 else: 4, not false]");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 3);
        REQUIRE(arr[0]->is<frst::Int>());
        REQUIRE(arr[1]->is<frst::Int>());
        REQUIRE(arr[2]->is<frst::Bool>());
        CHECK(arr[0]->get<frst::Int>().value() == 3_f);
        CHECK(arr[1]->get<frst::Int>().value() == 3_f);
        CHECK(arr[2]->get<frst::Bool>().value() == true);
    }

    SECTION("Array element expressions can span newlines")
    {
        auto result = parse("[1 +\n 2]");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 1);
        REQUIRE(arr[0]->is<frst::Int>());
        CHECK(arr[0]->get<frst::Int>().value() == 3_f);

        auto result2 = parse("[1 + # comment\n 2]");
        REQUIRE(result2.has_value());
        auto expr2 = std::move(result2).value();
        auto out2 = expr2->evaluate(ctx);
        REQUIRE(out2->is<frst::Array>());
        const auto& arr2 = out2->raw_get<frst::Array>();
        REQUIRE(arr2.size() == 1);
        REQUIRE(arr2[0]->is<frst::Int>());
        CHECK(arr2[0]->get<frst::Int>().value() == 3_f);
    }

    SECTION("Array literals can be passed as function arguments")
    {
        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto callable = std::make_shared<IdentityCallable>();
        table.define("id", frst::Value::create(frst::Function{callable}));

        auto result = parse("id([1, 2])");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 2);
        CHECK(arr[0]->get<frst::Int>().value() == 1_f);
        CHECK(arr[1]->get<frst::Int>().value() == 2_f);
    }

    SECTION("Array literals can be used as index expressions")
    {
        auto result = parse("arr[[0]]");
        REQUIRE(result.has_value());
    }

    SECTION("Array literals can participate in postfix and prefix")
    {
        auto result = parse("-[1, 2][0]");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == -1_f);
    }

    SECTION("Array literals can be used in conditions and unary ops")
    {
        auto result = parse("if [1]: 2 else: 3");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 2_f);

        auto result2 = parse("not [1]");
        REQUIRE(result2.has_value());
        auto expr2 = std::move(result2).value();
        auto out2 = expr2->evaluate(ctx);
        REQUIRE(out2->is<frst::Bool>());
        CHECK(out2->get<frst::Bool>().value() == false);
    }

    SECTION("Arrays can start statements in programs")
    {
        auto program_result = frst::parse_program(std::string{"[]\n42"}, "<test>");
        REQUIRE(program_result.has_value());
        auto program = std::move(program_result).value();
        REQUIRE(program.size() == 2);

        frst::Symbol_Table table;
        auto first = evaluate_expression(program[0], table);
        auto second = evaluate_expression(program[1], table);
        REQUIRE(first->is<frst::Array>());
        REQUIRE(second->is<frst::Int>());
        CHECK(second->get<frst::Int>().value() == 42_f);
    }

    SECTION("Trailing comma with comments is accepted")
    {
        auto result = parse("[1, 2, # c\n]");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 2);
    }

    SECTION("Malformed arrays are rejected")
    {
        CHECK(not parse("[1,2"));
        CHECK(not parse("[1,,2]"));
        CHECK(not parse("[,]"));
        CHECK(not parse("[,1]"));
        CHECK(not parse("[1, ,2]"));
        CHECK(not parse("[1,,]"));
        CHECK(not parse("[1,2,,]"));
        CHECK(not parse("[1;2]"));
        CHECK(not parse("[1; 2, 3]"));
        CHECK(not parse("[1, 2; 3]"));
        CHECK(not parse("[1;]"));
        CHECK(not parse("[;1]"));
    }

    SECTION("Source ranges for array literals")
    {
        // "[]" → begin at '[' {1,1}, end at ']' {1,2}
        auto result = parse("[]");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        auto range = expr->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 1);
        CHECK(range.end.line == 1);
        CHECK(range.end.column == 2);
    }

    SECTION("Source ranges for non-empty arrays")
    {
        // "[1, 2, 3]" → begin at '[' {1,1}, end at ']' {1,9}
        auto result = parse("[1, 2, 3]");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        auto range = expr->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 1);
        CHECK(range.end.line == 1);
        CHECK(range.end.column == 9);
    }
}
