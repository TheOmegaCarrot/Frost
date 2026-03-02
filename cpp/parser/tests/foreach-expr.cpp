#include <catch2/catch_test_macros.hpp>

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

struct RecordingCallable final : frst::Callable
{
    explicit RecordingCallable(bool return_value = false)
        : return_value{return_value}
    {
    }

    mutable std::vector<std::vector<frst::Value_Ptr>> calls;
    bool return_value;

    frst::Value_Ptr call(std::span<const frst::Value_Ptr> args) const override
    {
        calls.emplace_back(args.begin(), args.end());
        return frst::Value::create(return_value);
    }

    std::string debug_dump() const override
    {
        return "<recording>";
    }
};
} // namespace

TEST_CASE("Parser Foreach Expressions")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        auto src = lexy::string_input<lexy::utf8_encoding>(input);
        return lexy::parse<Expression_Root>(src, lexy::noop);
    };

    SECTION("Foreach over arrays calls the operation")
    {
        auto result = parse("foreach [1, 2, 3] with f");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto callable = std::make_shared<RecordingCallable>();
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Null>());
        REQUIRE(callable->calls.size() == 3);
        CHECK(callable->calls[0][0]->get<frst::Int>().value() == 1_f);
        CHECK(callable->calls[1][0]->get<frst::Int>().value() == 2_f);
        CHECK(callable->calls[2][0]->get<frst::Int>().value() == 3_f);
    }

    SECTION("Foreach over maps calls the operation with key and value")
    {
        auto result = parse("foreach {a: 1, b: 2} with f");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto callable = std::make_shared<RecordingCallable>();
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Null>());
        REQUIRE(callable->calls.size() == 2);
        for (const auto& call : callable->calls)
        {
            REQUIRE(call.size() == 2);
        }

        bool saw_a = false;
        bool saw_b = false;

        for (const auto& call : callable->calls)
        {
            if (call[0]->is<frst::String>()
                && call[0]->get<frst::String>().value()
                == "a")
            {
                saw_a = true;
                CHECK(call[1]->get<frst::Int>().value() == 1_f);
            }
            if (call[0]->is<frst::String>()
                && call[0]->get<frst::String>().value()
                == "b")
            {
                saw_b = true;
                CHECK(call[1]->get<frst::Int>().value() == 2_f);
            }
        }

        CHECK(saw_a);
        CHECK(saw_b);
    }

    SECTION("Foreach can iterate over a filtered array")
    {
        auto result =
            parse("foreach (filter [1, 2, 3] with fn (x) -> { x > 1 }) with f");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto callable = std::make_shared<RecordingCallable>();
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Null>());
        REQUIRE(callable->calls.size() == 2);
        CHECK(callable->calls[0][0]->get<frst::Int>().value() == 2_f);
        CHECK(callable->calls[1][0]->get<frst::Int>().value() == 3_f);
    }

    SECTION("Foreach can iterate over a mapped map expression")
    {
        auto result = parse(
            "foreach (map {a: 1, b: 2} with fn (k, v) -> { {[k]: v + 1} }) "
            "with f");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto callable = std::make_shared<RecordingCallable>();
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Null>());
        REQUIRE(callable->calls.size() == 2);

        bool saw_a = false;
        bool saw_b = false;

        for (const auto& call : callable->calls)
        {
            REQUIRE(call.size() == 2);
            if (call[0]->is<frst::String>()
                && call[0]->get<frst::String>().value()
                == "a")
            {
                saw_a = true;
                CHECK(call[1]->get<frst::Int>().value() == 2_f);
            }
            if (call[0]->is<frst::String>()
                && call[0]->get<frst::String>().value()
                == "b")
            {
                saw_b = true;
                CHECK(call[1]->get<frst::Int>().value() == 3_f);
            }
        }

        CHECK(saw_a);
        CHECK(saw_b);
    }

    SECTION("Foreach over arrays stops when operation returns truthy")
    {
        auto result = parse("foreach [1, 2, 3] with f");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto callable = std::make_shared<RecordingCallable>(true);
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Null>());
        REQUIRE(callable->calls.size() == 1);
        REQUIRE(callable->calls[0].size() == 1);
        CHECK(callable->calls[0][0]->get<frst::Int>().value() == 1_f);
    }

    SECTION("Foreach over maps stops when operation returns truthy")
    {
        auto result = parse("foreach {a: 1, b: 2} with f");
        REQUIRE(result);
        auto expr = require_expression(result);

        frst::Symbol_Table table;
        auto callable = std::make_shared<RecordingCallable>(true);
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto out = expr->evaluate(table);
        REQUIRE(out->is<frst::Null>());
        REQUIRE(callable->calls.size() == 1);
        REQUIRE(callable->calls[0].size() == 2);
    }

    SECTION("Invalid foreach expressions fail to parse")
    {
        const std::string_view cases[] = {
            "foreach with f",
            "foreach [1] f",
            "foreach [1] with",
            "foreach [1] with init",
        };

        for (const auto& input : cases)
        {
            auto result = parse(input);
            CHECK_FALSE(result);
            CHECK(result.error_count() >= 1);
        }
    }
}
