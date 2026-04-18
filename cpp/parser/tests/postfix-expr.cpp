#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

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
    mutable std::vector<frst::Value_Ptr> received;
    frst::Value_Ptr result;

    frst::Value_Ptr call(std::span<const frst::Value_Ptr> args) const override
    {
        received.assign(args.begin(), args.end());
        return result ? result : frst::Value::null();
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
} // namespace

TEST_CASE("Parser Postfix Expressions")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        return frst::parse_data(std::string{input});
    };

    SECTION("Indexing parses and evaluates")
    {
        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};

        auto arr_val = frst::Value::create(frst::Array{
            frst::Value::create(1_f),
            frst::Value::create(2_f),
            frst::Value::create(3_f),
        });
        table.define("arr", arr_val);

        frst::Map map;
        map.emplace(frst::Value::create(std::string{"key"}),
                    frst::Value::create(42_f));
        table.define("m", frst::Value::create(std::move(map)));

        auto arr_result = parse("arr[1]");
        REQUIRE(arr_result.has_value());
        auto arr_expr = std::move(arr_result).value();
        auto arr_out = arr_expr->evaluate(ctx);
        REQUIRE(arr_out->is<frst::Int>());
        CHECK(arr_out->get<frst::Int>().value() == 2_f);

        auto arr_expr_result = parse("arr[1+1]");
        REQUIRE(arr_expr_result.has_value());
        auto arr_expr2 = std::move(arr_expr_result).value();
        auto arr_out2 = arr_expr2->evaluate(ctx);
        REQUIRE(arr_out2->is<frst::Int>());
        CHECK(arr_out2->get<frst::Int>().value() == 3_f);

        auto arr_neg_result = parse("arr[-1]");
        REQUIRE(arr_neg_result.has_value());
        auto arr_expr3 = std::move(arr_neg_result).value();
        auto arr_out3 = arr_expr3->evaluate(ctx);
        REQUIRE(arr_out3->is<frst::Int>());
        CHECK(arr_out3->get<frst::Int>().value() == 3_f);

        auto map_result = parse("m[\"key\"]");
        REQUIRE(map_result.has_value());
        auto map_expr = std::move(map_result).value();
        auto map_out = map_expr->evaluate(ctx);
        REQUIRE(map_out->is<frst::Int>());
        CHECK(map_out->get<frst::Int>().value() == 42_f);
    }

    SECTION("Indexing allows newlines inside brackets")
    {
        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};

        auto arr_val = frst::Value::create(frst::Array{
            frst::Value::create(11_f),
            frst::Value::create(22_f),
        });
        table.define("arr", arr_val);

        auto result = parse("arr[\n0\n]");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 11_f);
    }

    SECTION("Dot access parses as string indexing")
    {
        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};

        frst::Map inner;
        inner.emplace(frst::Value::create(std::string{"value"}),
                      frst::Value::create(99_f));

        frst::Map outer;
        outer.emplace(frst::Value::create(std::string{"inner"}),
                      frst::Value::create(std::move(inner)));
        outer.emplace(frst::Value::create(std::string{"key"}),
                      frst::Value::create(7_f));
        table.define("obj", frst::Value::create(std::move(outer)));

        auto dot_result = parse("obj.key");
        REQUIRE(dot_result.has_value());
        auto dot_expr = std::move(dot_result).value();
        auto dot_out = dot_expr->evaluate(ctx);
        REQUIRE(dot_out->is<frst::Int>());
        CHECK(dot_out->get<frst::Int>().value() == 7_f);

        auto chain_result = parse("obj.inner.value");
        REQUIRE(chain_result.has_value());
        auto chain_expr = std::move(chain_result).value();
        auto chain_out = chain_expr->evaluate(ctx);
        REQUIRE(chain_out->is<frst::Int>());
        CHECK(chain_out->get<frst::Int>().value() == 99_f);
    }

    SECTION("Dot access uses lookup semantics and can return Null")
    {
        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};

        frst::Map obj;
        obj.emplace(frst::Value::create(std::string{"present"}),
                    frst::Value::create(1_f));
        table.define("obj", frst::Value::create(std::move(obj)));

        auto result = parse("obj.missing");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Null>());
    }

    SECTION("Function call parses and evaluates")
    {
        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};

        auto callable = std::make_shared<RecordingCallable>();
        callable->result = frst::Value::create(123_f);
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto call_result = parse("f(1, 2+3)");
        REQUIRE(call_result.has_value());
        auto call_expr = std::move(call_result).value();
        auto out = call_expr->evaluate(ctx);

        CHECK(out == callable->result);
        REQUIRE(callable->received.size() == 2);
        REQUIRE(callable->received.at(0)->is<frst::Int>());
        REQUIRE(callable->received.at(1)->is<frst::Int>());
        CHECK(callable->received.at(0)->get<frst::Int>().value() == 1_f);
        CHECK(callable->received.at(1)->get<frst::Int>().value() == 5_f);
    }

    SECTION("Empty call argument list is permitted")
    {
        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};

        auto callable = std::make_shared<RecordingCallable>();
        callable->result = frst::Value::create(5_f);
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto call_result = parse("f()");
        REQUIRE(call_result.has_value());
        auto call_expr = std::move(call_result).value();
        auto out = call_expr->evaluate(ctx);

        CHECK(out == callable->result);
        CHECK(callable->received.empty());
    }

    SECTION("Call arguments allow newlines inside parentheses")
    {
        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};

        auto callable = std::make_shared<RecordingCallable>();
        callable->result = frst::Value::create(7_f);
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto call_result = parse("f(\n1,\n2\n)");
        REQUIRE(call_result.has_value());
        auto call_expr = std::move(call_result).value();
        auto out = call_expr->evaluate(ctx);

        CHECK(out == callable->result);
        REQUIRE(callable->received.size() == 2);
        CHECK(callable->received[0]->get<frst::Int>().value() == 1_f);
        CHECK(callable->received[1]->get<frst::Int>().value() == 2_f);

        callable->received.clear();
        auto empty_result = parse("f(\n)");
        REQUIRE(empty_result.has_value());
        auto empty_expr = std::move(empty_result).value();
        auto out2 = empty_expr->evaluate(ctx);

        CHECK(out2 == callable->result);
        CHECK(callable->received.empty());
    }

    SECTION("Call arguments reject trailing commas and empty slots")
    {
        CHECK(not parse("f(1,2,)"));
        CHECK(not parse("f(,)"));
        CHECK(not parse("f(,1)"));
    }

    SECTION("Whitespace around postfix tokens is allowed")
    {
        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};

        auto arr_val = frst::Value::create(frst::Array{
            frst::Value::create(5_f),
        });
        table.define("arr", arr_val);

        frst::Map obj;
        obj.emplace(frst::Value::create(std::string{"key"}),
                    frst::Value::create(8_f));
        table.define("obj", frst::Value::create(std::move(obj)));

        auto callable = std::make_shared<RecordingCallable>();
        callable->result = frst::Value::create(9_f);
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto index_result = parse("arr [ 0 ]");
        REQUIRE(index_result.has_value());
        auto index_expr = std::move(index_result).value();
        auto index_out = index_expr->evaluate(ctx);
        REQUIRE(index_out->is<frst::Int>());
        CHECK(index_out->get<frst::Int>().value() == 5_f);

        auto dot_result = parse("obj . key");
        REQUIRE(dot_result.has_value());
        auto dot_expr = std::move(dot_result).value();
        auto dot_out = dot_expr->evaluate(ctx);
        REQUIRE(dot_out->is<frst::Int>());
        CHECK(dot_out->get<frst::Int>().value() == 8_f);

        auto call_result = parse("f ( 1 , 2 )");
        REQUIRE(call_result.has_value());
        auto call_expr = std::move(call_result).value();
        auto call_out = call_expr->evaluate(ctx);
        CHECK(call_out == callable->result);
    }

    SECTION("Postfix chaining binds tighter than binary operators")
    {
        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};

        auto callable = std::make_shared<IdentityCallable>();
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto result = parse("1 + f(2)");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        auto out = expr->evaluate(ctx);

        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 3_f);
    }

    SECTION("Postfix chaining with dot then call")
    {
        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};

        auto callable = std::make_shared<RecordingCallable>();
        callable->result = frst::Value::create(4_f);

        frst::Map obj;
        obj.emplace(frst::Value::create(std::string{"f"}),
                    frst::Value::create(frst::Function{callable}));
        table.define("obj", frst::Value::create(std::move(obj)));

        auto result = parse("obj.f(3)");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        auto out = expr->evaluate(ctx);

        CHECK(out == callable->result);
        REQUIRE(callable->received.size() == 1);
        REQUIRE(callable->received.at(0)->is<frst::Int>());
        CHECK(callable->received.at(0)->get<frst::Int>().value() == 3_f);
    }

    SECTION("Postfix chaining with index then call")
    {
        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};

        auto callable = std::make_shared<RecordingCallable>();
        callable->result = frst::Value::create(7_f);

        frst::Array arr;
        arr.push_back(frst::Value::create(frst::Function{callable}));
        table.define("arr", frst::Value::create(std::move(arr)));

        auto result = parse("arr[0](2)");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        auto out = expr->evaluate(ctx);

        CHECK(out == callable->result);
        REQUIRE(callable->received.size() == 1);
        REQUIRE(callable->received.at(0)->is<frst::Int>());
        CHECK(callable->received.at(0)->get<frst::Int>().value() == 2_f);
    }

    SECTION("Call then index is supported")
    {
        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};

        auto callable = std::make_shared<RecordingCallable>();
        auto array_val = frst::Value::create(frst::Array{
            frst::Value::create(10_f),
            frst::Value::create(20_f),
        });
        callable->result = array_val;
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto result = parse("f()[0]");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        auto out = expr->evaluate(ctx);

        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 10_f);
    }

    SECTION("Deep postfix chains parse and evaluate")
    {
        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};

        auto callable = std::make_shared<RecordingCallable>();
        frst::Map inner;
        inner.emplace(frst::Value::create(std::string{"key"}),
                      frst::Value::create(77_f));
        frst::Array arr;
        arr.push_back(frst::Value::create(std::move(inner)));
        callable->result = frst::Value::create(std::move(arr));

        frst::Map obj;
        obj.emplace(frst::Value::create(std::string{"f"}),
                    frst::Value::create(frst::Function{callable}));
        table.define("obj", frst::Value::create(std::move(obj)));

        auto result = parse("obj.f()[0].key");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        auto out = expr->evaluate(ctx);

        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 77_f);
    }

    SECTION("Pathological whitespace and comments in postfix chains")
    {
        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};

        auto arr_val = frst::Value::create(frst::Array{
            frst::Value::create(1_f),
            frst::Value::create(2_f),
            frst::Value::create(3_f),
        });
        table.define("arr", arr_val);

        frst::Map inner;
        inner.emplace(frst::Value::create(std::string{"value"}),
                      frst::Value::create(10_f));

        frst::Map obj;
        obj.emplace(frst::Value::create(std::string{"key"}),
                    frst::Value::create(7_f));
        obj.emplace(frst::Value::create(std::string{"inner"}),
                    frst::Value::create(std::move(inner)));
        table.define("obj", frst::Value::create(std::move(obj)));

        auto callable = std::make_shared<RecordingCallable>();
        callable->result = frst::Value::create(42_f);
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto index_comment = parse("arr[ # comment\n 1 ]");
        REQUIRE(index_comment.has_value());
        auto index_expr = std::move(index_comment).value();
        auto index_out = index_expr->evaluate(ctx);
        REQUIRE(index_out->is<frst::Int>());
        CHECK(index_out->get<frst::Int>().value() == 2_f);

        auto index_newlines = parse("arr[\n -1 \n]");
        REQUIRE(index_newlines.has_value());
        auto index_expr2 = std::move(index_newlines).value();
        auto index_out2 = index_expr2->evaluate(ctx);
        REQUIRE(index_out2->is<frst::Int>());
        CHECK(index_out2->get<frst::Int>().value() == 3_f);

        CHECK(not frst::parse_program(std::string{"obj.\nkey"}));
        CHECK(not frst::parse_program(std::string{"obj.# comment\nkey"}));
        CHECK(not frst::parse_program(std::string{"obj .\n inner \n.\n value"}));

        auto call_empty = parse("f( # comment\n )");
        REQUIRE(call_empty.has_value());
        auto call_expr = std::move(call_empty).value();
        auto call_out = call_expr->evaluate(ctx);
        CHECK(call_out == callable->result);
        CHECK(callable->received.empty());

        auto call_split = parse("f(\n 1 # comment\n ,\n 2\n)");
        REQUIRE(call_split.has_value());
        auto call_expr2 = std::move(call_split).value();
        auto call_out2 = call_expr2->evaluate(ctx);
        CHECK(call_out2 == callable->result);
        REQUIRE(callable->received.size() == 2);
        REQUIRE(callable->received.at(0)->is<frst::Int>());
        REQUIRE(callable->received.at(1)->is<frst::Int>());
        CHECK(callable->received.at(0)->get<frst::Int>().value() == 1_f);
        CHECK(callable->received.at(1)->get<frst::Int>().value() == 2_f);
    }

    SECTION("Parenthesized base supports postfix chaining")
    {
        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};

        frst::Map obj;
        obj.emplace(frst::Value::create(std::string{"key"}),
                    frst::Value::create(6_f));
        table.define("obj", frst::Value::create(std::move(obj)));

        auto callable = std::make_shared<RecordingCallable>();
        callable->result = frst::Value::create(13_f);
        table.define("f", frst::Value::create(frst::Function{callable}));

        auto index_result = parse("(obj).key");
        REQUIRE(index_result.has_value());
        auto index_expr = std::move(index_result).value();
        auto index_out = index_expr->evaluate(ctx);
        REQUIRE(index_out->is<frst::Int>());
        CHECK(index_out->get<frst::Int>().value() == 6_f);

        auto call_result = parse("((f))(1)");
        REQUIRE(call_result.has_value());
        auto call_expr = std::move(call_result).value();
        auto call_out = call_expr->evaluate(ctx);
        CHECK(call_out == callable->result);
    }

    SECTION("Invalid call syntax fails to parse")
    {
        CHECK(not parse("f(1,)"));
        CHECK(not parse("f(,1)"));
        CHECK(not parse("f(1 2)"));
        CHECK(not parse("f(1"));
        CHECK(not parse("f(1,,2)"));
        CHECK(not parse("f(,)"));
        CHECK(not parse("f(,1,)"));
    }

    SECTION("Invalid index syntax fails to parse")
    {
        CHECK(not parse("arr[]"));
        CHECK(not parse("arr[1"));
    }

    SECTION("Invalid dot access fails to parse")
    {
        CHECK(not parse("obj."));
        CHECK(not parse("obj.if"));
        CHECK(not parse("obj.1"));
        CHECK(not parse("obj.true"));
        CHECK(not parse("obj.false"));
        CHECK(not parse("obj.and"));
        CHECK(not parse("obj.or"));
        CHECK(not parse("obj.not"));
        CHECK(not parse("obj.null"));
    }

    SECTION("Malformed postfix chains fail to parse")
    {
        const std::string_view cases[] = {
            "obj..key",
            "obj.[0]",
            "obj.(1)",
            "obj.# comment\n",
            "arr[]",
            "arr[ ]",
            "arr[ # comment\n ]",
            "arr[0,1]",
            "arr[0]]",
            "arr[0][)",
            "arr[)",
            "f(1,2))",
            "f((1,2))",
            "f(1,2,,3)",
            "f(1)(,2)",
            "f(,)",
            "f(1,2,)",
        };

        for (const auto& input : cases)
        {
            CHECK(not parse(input));
        }
    }

    SECTION("Source ranges for index expression")
    {
        // "arr[1]" → begin at 'a' {1,1}, end at ']' {1,6}
        auto result = parse("arr[1]");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        auto range = expr->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 1);
        CHECK(range.end.line == 1);
        CHECK(range.end.column == 6);
    }

    SECTION("Source ranges for dot access")
    {
        // "obj.key" → begin at 'o' {1,1}, end at 'y' {1,7}
        auto result = parse("obj.key");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        auto range = expr->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 1);
        CHECK(range.end.line == 1);
        CHECK(range.end.column == 7);
    }

    SECTION("Source ranges for function call")
    {
        // "f(1, 2)" → begin at 'f' {1,1}, end at ')' {1,7}
        auto result = parse("f(1, 2)");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        auto range = expr->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 1);
        CHECK(range.end.line == 1);
        CHECK(range.end.column == 7);
    }

    SECTION("Source ranges for chained postfix")
    {
        // "obj.f(3)" → begin at 'o' {1,1}, end at ')' {1,8}
        auto result = parse("obj.f(3)");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        auto range = expr->source_range();
        CHECK(range.begin.line == 1);
        CHECK(range.begin.column == 1);
        CHECK(range.end.line == 1);
        CHECK(range.end.column == 8);
    }

    SECTION("Postfix source ranges exclude trailing whitespace")
    {
        // "x[0] + 1": index expr 'x[0]' must end at 4, not 5
        auto result = parse("x[0] + 1");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        auto nodes = expr->walk() | std::ranges::to<std::vector>();
        REQUIRE(nodes.size() >= 2);

        auto lhs_range = nodes[1]->source_range();
        CHECK(lhs_range.begin.column == 1);
        CHECK(lhs_range.end.column == 4);
    }

    SECTION("Dot access source range excludes trailing whitespace")
    {
        // "obj.key + 1": dot access 'obj.key' must end at 7, not 8
        auto result = parse("obj.key + 1");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        auto nodes = expr->walk() | std::ranges::to<std::vector>();
        REQUIRE(nodes.size() >= 2);

        auto lhs_range = nodes[1]->source_range();
        CHECK(lhs_range.begin.column == 1);
        CHECK(lhs_range.end.column == 7);
    }

    SECTION("Call source range excludes trailing whitespace")
    {
        // "f(1) + 2": call 'f(1)' must end at 4, not 5
        auto result = parse("f(1) + 2");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        auto nodes = expr->walk() | std::ranges::to<std::vector>();
        REQUIRE(nodes.size() >= 2);

        auto lhs_range = nodes[1]->source_range();
        CHECK(lhs_range.begin.column == 1);
        CHECK(lhs_range.end.column == 4);
    }

    SECTION("Dot access key literal gets source range of identifier")
    {
        // "foo.bar": Index [1:1-1:7], key Literal("bar") [1:5-1:7]
        auto result = parse("foo.bar");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        CHECK(expr->source_range().begin.column == 1);
        CHECK(expr->source_range().end.column == 7);

        // walk: [0]=Index, [1]=Name_Lookup(foo), [2]=Literal("bar")
        auto nodes = expr->walk() | std::ranges::to<std::vector>();
        REQUIRE(nodes.size() == 3);

        auto key_range = nodes[2]->source_range();
        CHECK(key_range.begin.line == 1);
        CHECK(key_range.begin.column == 5);
        CHECK(key_range.end.line == 1);
        CHECK(key_range.end.column == 7);
    }

    SECTION("Dot key range excludes trailing whitespace")
    {
        // "x.y + 1": key 'y' must be [1:3-1:3], not extending into the space
        auto result = parse("x.y + 1");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        // walk into the Binop LHS → Index → key
        auto top_nodes = expr->walk() | std::ranges::to<std::vector>();
        // [0]=Binop, [1]=Index(x.y), [2]=Name_Lookup(x), [3]=Literal("y"),
        // [4]=Literal(1)
        REQUIRE(top_nodes.size() == 5);

        auto key_range = top_nodes[3]->source_range();
        CHECK(key_range.begin.column == 3);
        CHECK(key_range.end.column == 3);
    }

    SECTION("Chained dot key ranges")
    {
        // "a.b.c": each key literal gets its own range
        auto result = parse("a.b.c");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        CHECK(expr->source_range().begin.column == 1);
        CHECK(expr->source_range().end.column == 5);

        // walk: [0]=Index(a.b.c), [1]=Index(a.b), [2]=Name_Lookup(a),
        //       [3]=Literal("b"), [4]=Literal("c")
        auto nodes = expr->walk() | std::ranges::to<std::vector>();
        REQUIRE(nodes.size() == 5);

        auto b_range = nodes[3]->source_range();
        CHECK(b_range.begin.column == 3);
        CHECK(b_range.end.column == 3);

        auto c_range = nodes[4]->source_range();
        CHECK(c_range.begin.column == 5);
        CHECK(c_range.end.column == 5);
    }
}
