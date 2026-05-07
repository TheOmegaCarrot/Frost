#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/ast.hpp>
#include <frost/parser.hpp>
#include <frost/symbol-table.hpp>
#include <frost/testing/stringmaker-specializations.hpp>
#include <frost/value.hpp>

using namespace frst::literals;

TEST_CASE("Parser Reduce Expressions")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        return frst::parse_data(std::string{input});
    };

    SECTION("Reduce array with init")
    {
        auto result =
            parse("reduce [1, 2, 3] init: 0 with fn (acc, x) -> { acc + x }");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 6_f);
    }

    SECTION("Reduce array without init")
    {
        auto result = parse("reduce [1, 2, 3] with fn (acc, x) -> { acc + x }");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 6_f);
    }

    SECTION("Reduce empty array with init returns init")
    {
        auto result =
            parse("reduce [] init: 5 with fn (acc, x) -> { acc + x }");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 5_f);
    }

    SECTION("Reduce map without init is an evaluation error")
    {
        auto result = parse("reduce {a: 1} with fn (acc, k, v) -> { acc }");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        CHECK_THROWS_WITH(expr->evaluate(ctx),
                          Catch::Matchers::ContainsSubstring("init"));
    }

    SECTION("Reduce expressions can participate in larger expressions")
    {
        auto result =
            parse("(reduce [1, 2] with fn (acc, x) -> { acc + x }) + 1");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 4_f);
    }

    SECTION("Reduce results can be chained in larger expressions")
    {
        auto result =
            parse("(reduce [1, 2, 3] with fn (acc, x) -> { acc + x }) * 2");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 12_f);

        auto result2 =
            parse("(reduce [] init: 4 with fn (acc, x) -> { acc + x }) - 1");
        REQUIRE(result2.has_value());
        auto expr2 = std::move(result2).value();
        auto out2 = expr2->evaluate(ctx);
        REQUIRE(out2->is<frst::Int>());
        CHECK(out2->get<frst::Int>().value() == 3_f);

        auto result3 =
            parse("(reduce {a: 1} init: 5 with fn (acc, k, v) -> { acc }) + 1");
        REQUIRE(result3.has_value());
        auto expr3 = std::move(result3).value();
        auto out3 = expr3->evaluate(ctx);
        REQUIRE(out3->is<frst::Int>());
        CHECK(out3->get<frst::Int>().value() == 6_f);
    }

    SECTION("Whitespace and comments are allowed around reduce init")
    {
        auto result =
            parse("reduce [1, 2] init:\n0 with fn (acc, x) -> { acc + x }");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 3_f);

        auto result2 =
            parse("reduce [1, 2] init : 0 with fn (acc, x) -> { acc + x }");
        REQUIRE(result2.has_value());
        auto expr2 = std::move(result2).value();
        auto out2 = expr2->evaluate(ctx);
        REQUIRE(out2->is<frst::Int>());
        CHECK(out2->get<frst::Int>().value() == 3_f);
    }

    SECTION("Reduce expressions can appear inside other constructs")
    {
        auto result = parse("[reduce [1, 2] with fn (acc, x) -> { acc + x }]");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 1);
        CHECK(arr[0]->get<frst::Int>().value() == 3_f);

        auto result2 =
            parse("{[reduce [1, 2] with fn (acc, x) -> { acc + x }]: 1}");
        REQUIRE(result2.has_value());
        auto expr2 = std::move(result2).value();
        auto out2 = expr2->evaluate(ctx);
        REQUIRE(out2->is<frst::Map>());
        auto key = frst::Value::create(3_f);
        auto it = out2->raw_get<frst::Map>().find(key);
        REQUIRE(it != out2->raw_get<frst::Map>().end());

        auto result3 = parse(
            "if reduce [1, 2] with fn (acc, x) -> { acc + x }: 1 else: 2");
        REQUIRE(result3.has_value());
        auto expr3 = std::move(result3).value();
        auto out3 = expr3->evaluate(ctx);
        REQUIRE(out3->is<frst::Int>());
        CHECK(out3->get<frst::Int>().value() == 1_f);
    }

    SECTION("Reduce does not bind across newlines in larger expressions")
    {
        auto result = frst::parse_program(
            std::string{"(reduce [1, 2] with fn (acc, x) -> { acc + x })\n1"}, "<test>");
        REQUIRE(result.has_value());
        CHECK(result.value().size() == 2);
    }

    SECTION("Reduce expressions can nest other higher-order expressions")
    {
        auto result = parse("reduce (map [1, 2] with fn (x) -> { x }) "
                            "with fn (acc, x) -> { acc + x }");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 3_f);

        auto result2 = parse("reduce [1, 2] with fn (acc, x) -> { "
                             "(map [x] with fn (y) -> { y })[0] + acc }");
        REQUIRE(result2.has_value());
        auto expr2 = std::move(result2).value();
        auto out2 = expr2->evaluate(ctx);
        REQUIRE(out2->is<frst::Int>());
        CHECK(out2->get<frst::Int>().value() == 3_f);

        auto result3 = parse("reduce [1, 2] "
                             "init: (map [3] with fn (x) -> { x })[0] "
                             "with fn (acc, x) -> { acc + x }");
        REQUIRE(result3.has_value());
        auto expr3 = std::move(result3).value();
        auto out3 = expr3->evaluate(ctx);
        REQUIRE(out3->is<frst::Int>());
        CHECK(out3->get<frst::Int>().value() == 6_f);

        auto result4 =
            parse("reduce {a: 1} "
                  "init: (reduce [1, 2] with fn (acc, x) -> { acc + x }) "
                  "with fn (acc, k, v) -> { acc }");
        REQUIRE(result4.has_value());
        auto expr4 = std::move(result4).value();
        auto out4 = expr4->evaluate(ctx);
        REQUIRE(out4->is<frst::Int>());
        CHECK(out4->get<frst::Int>().value() == 3_f);
    }

    SECTION("Init expressions can include nested map and threaded calls")
    {
        struct Identity_Callable final : frst::Callable
        {
            frst::Value_Ptr call(
                std::span<const frst::Value_Ptr> args) const override
            {
                if (args.empty())
                    return frst::Value::null();
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

        auto result = parse("reduce [1] "
                            "init: (map [1] with f @ g())[0] "
                            "with fn (acc, x) -> { acc + x }");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto f_callable = std::make_shared<Identity_Callable>();
        auto g_callable = std::make_shared<Identity_Callable>();
        table.define("f", frst::Value::create(frst::Function{f_callable}));
        table.define("g", frst::Value::create(frst::Function{g_callable}));

        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 2_f);
    }

    SECTION("Reduce expressions can be passed as call arguments")
    {
        struct IdentityCallable final : frst::Callable
        {
            frst::Value_Ptr call(
                std::span<const frst::Value_Ptr> args) const override
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

        auto result =
            parse("f(reduce [1, 2, 3] with fn (acc, x) -> { acc + x })");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto callable = std::make_shared<IdentityCallable>();
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 6_f);
    }

    SECTION("Threaded call can apply to reduce expressions")
    {
        struct IdentityCallable final : frst::Callable
        {
            frst::Value_Ptr call(
                std::span<const frst::Value_Ptr> args) const override
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

        auto result =
            parse("reduce [1, 2] with fn (acc, x) -> { acc + x } @ f()");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto callable = std::make_shared<IdentityCallable>();
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 3_f);
    }

    SECTION("Reduce can consume a filtered structure and map init")
    {
        auto result =
            parse("reduce (filter [1, 2, 3] with fn (x) -> { x > 1 }) "
                  "init: (map [1] with fn (x) -> { x })[0] "
                  "with fn (acc, x) -> { acc + x }");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 6_f);
    }

    SECTION("Nested reduce in operation expression is parsed correctly")
    {
        auto result = parse("reduce a with reduce b init: c with f");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        struct Return_Acc final : frst::Callable
        {
            frst::Value_Ptr call(
                std::span<const frst::Value_Ptr> args) const override
            {
                return args.front();
            }

            std::string debug_dump() const override
            {
                return "<return-acc>";
            }
            std::string name() const override
            {
                return debug_dump();
            }
        };

        struct Recording_Acc final : frst::Callable
        {
            mutable std::vector<std::vector<frst::Value_Ptr>> calls;

            frst::Value_Ptr call(
                std::span<const frst::Value_Ptr> args) const override
            {
                calls.emplace_back(args.begin(), args.end());
                return args.front();
            }

            std::string debug_dump() const override
            {
                return "<recording-acc>";
            }
            std::string name() const override
            {
                return debug_dump();
            }
        };

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto a_val = frst::Value::create(frst::Array{
            frst::Value::create(10_f),
            frst::Value::create(20_f),
        });
        auto b_val = frst::Value::create(frst::Array{
            frst::Value::create(1_f),
        });
        auto f_callable = std::make_shared<Return_Acc>();
        auto g_callable = std::make_shared<Recording_Acc>();
        auto c_val = frst::Value::create(frst::Function{g_callable});

        table.define("a", a_val);
        table.define("b", b_val);
        table.define("f", frst::Value::create(frst::Function{f_callable}));
        table.define("c", c_val);

        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 10_f);
        REQUIRE(g_callable->calls.size() == 1);
        CHECK(g_callable->calls[0][0]->get<frst::Int>().value() == 10_f);
        CHECK(g_callable->calls[0][1]->get<frst::Int>().value() == 20_f);
    }

    SECTION("Invalid reduce expressions fail to parse")
    {
        const std::string_view cases[] = {
            "reduce with f",
            "reduce [1] with f init: 0",
            "reduce [1] init 0 with f",
            "reduce [1] init: with f",
            "reduce [1] init: 0 init: 1 with f",
        };

        for (const auto& input : cases)
        {
            CHECK(not parse(input));
        }
    }

    SECTION("Source range for reduce without init")
    {
        // "reduce [1, 2] with fn (a, x) -> a + x" → [1:1-1:37]
        auto result = parse("reduce [1, 2] with fn (a, x) -> a + x");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        auto range = expr->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 1);
        CHECK(range.end.line == 1);
        CHECK(range.end.column == 37);
    }

    SECTION("Source range for reduce with init")
    {
        // "reduce [1] init: 0 with fn (a, x) -> a + x" -> [1:1-1:42]
        auto result = parse("reduce [1] init: 0 with fn (a, x) -> a + x");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        auto range = expr->source_range();
        CHECK(range.begin.column == 1);
        CHECK(range.end.column == 42);
    }

    SECTION("Source range excludes trailing whitespace")
    {
        auto result = parse("reduce [1] with fn (a, x) -> a + x   ");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        auto range = expr->source_range();
        CHECK(range.begin.column == 1);
        CHECK(range.end.column == 34);
    }
}
