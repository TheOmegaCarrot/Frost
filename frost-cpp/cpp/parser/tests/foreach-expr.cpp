#include <catch2/catch_test_macros.hpp>

#include <frost/ast.hpp>
#include <frost/parser.hpp>
#include <frost/symbol-table.hpp>
#include <frost/testing/stringmaker-specializations.hpp>
#include <frost/value.hpp>

using namespace frst::literals;

namespace
{
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
    std::string name() const override
    {
        return debug_dump();
    }
};
} // namespace

TEST_CASE("Parser Foreach Expressions")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        return frst::parse_data(std::string{input});
    };

    SECTION("Foreach over arrays calls the operation")
    {
        auto result = parse("foreach [1, 2, 3] with f");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto callable = std::make_shared<RecordingCallable>();
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Null>());
        REQUIRE(callable->calls.size() == 3);
        CHECK(callable->calls[0][0]->get<frst::Int>().value() == 1_f);
        CHECK(callable->calls[1][0]->get<frst::Int>().value() == 2_f);
        CHECK(callable->calls[2][0]->get<frst::Int>().value() == 3_f);
    }

    SECTION("Foreach over maps calls the operation with key and value")
    {
        auto result = parse("foreach {a: 1, b: 2} with f");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto callable = std::make_shared<RecordingCallable>();
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto out = expr->evaluate(ctx);
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
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto callable = std::make_shared<RecordingCallable>();
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto out = expr->evaluate(ctx);
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
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto callable = std::make_shared<RecordingCallable>();
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto out = expr->evaluate(ctx);
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

    SECTION("Foreach over arrays ignores return value")
    {
        auto result = parse("foreach [1, 2, 3] with f");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto callable = std::make_shared<RecordingCallable>(true);
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Null>());
        REQUIRE(callable->calls.size() == 3);
        CHECK(callable->calls[0][0]->get<frst::Int>().value() == 1_f);
        CHECK(callable->calls[1][0]->get<frst::Int>().value() == 2_f);
        CHECK(callable->calls[2][0]->get<frst::Int>().value() == 3_f);
    }

    SECTION("Foreach over maps ignores return value")
    {
        auto result = parse("foreach {a: 1, b: 2} with f");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto callable = std::make_shared<RecordingCallable>(true);
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Null>());
        REQUIRE(callable->calls.size() == 2);
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
            CHECK(not parse(input));
        }
    }

    SECTION("Source range for foreach expression")
    {
        // "foreach [1] with fn x -> x" → [1:1-1:26]
        auto result = parse("foreach [1] with fn x -> x");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        auto range = expr->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 1);
        CHECK(range.end.line == 1);
        CHECK(range.end.column == 26);
    }

    SECTION("Source range excludes trailing whitespace")
    {
        auto result = parse("foreach [1] with fn x -> x   ");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        auto range = expr->source_range();
        CHECK(range.begin.column == 1);
        CHECK(range.end.column == 26);
    }
}
