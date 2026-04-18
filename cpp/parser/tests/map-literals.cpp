#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/ast.hpp>
#include <frost/parser.hpp>
#include <frost/symbol-table.hpp>
#include <frost/testing/stringmaker-specializations.hpp>
#include <frost/value.hpp>

using namespace frst::literals;

TEST_CASE("Parser Map Literals")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        return frst::parse_data(std::string{input});
    };

    SECTION("Empty map literal is valid")
    {
        auto result = parse("{}");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Map>());
        CHECK(out->raw_get<frst::Map>().empty());
    }

    SECTION("Identifier key sugar produces string keys")
    {
        auto result = parse("{foo: 1, bar: 2}");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Map>());
        const auto& map = out->raw_get<frst::Map>();
        REQUIRE(map.size() == 2);

        auto foo_key = frst::Value::create(std::string{"foo"});
        auto bar_key = frst::Value::create(std::string{"bar"});
        auto foo_it = map.find(foo_key);
        auto bar_it = map.find(bar_key);
        REQUIRE(foo_it != map.end());
        REQUIRE(bar_it != map.end());
        CHECK(foo_it->second->get<frst::Int>().value() == 1_f);
        CHECK(bar_it->second->get<frst::Int>().value() == 2_f);
    }

    SECTION("Bracketed key expressions are supported")
    {
        auto result = parse("{[1]: 2, [1+1]: 3}");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Map>());
        const auto& map = out->raw_get<frst::Map>();
        REQUIRE(map.size() == 2);

        auto key1 = frst::Value::create(1_f);
        auto key2 = frst::Value::create(2_f);
        auto it1 = map.find(key1);
        auto it2 = map.find(key2);
        REQUIRE(it1 != map.end());
        REQUIRE(it2 != map.end());
        CHECK(it1->second->get<frst::Int>().value() == 2_f);
        CHECK(it2->second->get<frst::Int>().value() == 3_f);
    }

    SECTION("Mixed key forms are allowed")
    {
        auto result = parse("{foo: 1, [42]: 2, bar: 3}");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Map>());
        const auto& map = out->raw_get<frst::Map>();
        REQUIRE(map.size() == 3);
    }

    SECTION("Trailing commas are allowed")
    {
        auto result = parse("{foo: 1, [2]: 3,}");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Map>());
        CHECK(out->raw_get<frst::Map>().size() == 2);
    }

    SECTION("Commas are required between entries")
    {
        CHECK(not parse("{foo: 1 bar: 2}"));
        CHECK(not parse("{a: 1\nb:2}"));
    }

    SECTION("Newlines between entries are allowed with commas")
    {
        auto result = parse("{foo: 1,\nbar: 2}");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Map>());
        CHECK(out->raw_get<frst::Map>().size() == 2);
    }

    SECTION("Map value expressions can span newlines")
    {
        auto result = parse("{foo: 1 +\n 2}");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Map>());
        const auto& map = out->raw_get<frst::Map>();
        auto foo_key = frst::Value::create(std::string{"foo"});
        auto it = map.find(foo_key);
        REQUIRE(it != map.end());
        REQUIRE(it->second->is<frst::Int>());
        CHECK(it->second->get<frst::Int>().value() == 3_f);
    }

    SECTION("Map literals can appear inside arrays")
    {
        auto result = parse("[{a: 1}, {b: 2}]");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Array>());
        const auto& arr = out->raw_get<frst::Array>();
        REQUIRE(arr.size() == 2);
        REQUIRE(arr[0]->is<frst::Map>());
        REQUIRE(arr[1]->is<frst::Map>());
    }

    SECTION("Whitespace between % and { is not allowed")
    {
        CHECK(not parse("% {}"));
        CHECK(not parse("% {foo: 1}"));
    }

    SECTION("Identifier key cannot be a keyword")
    {
        CHECK(not parse("{if: 1}"));
        CHECK(not parse("{true: 1}"));
    }

    SECTION("Missing brackets for non-identifier keys is invalid")
    {
        CHECK(not parse("{1: 2}"));
        CHECK(not parse("{(a): 1}"));
    }

    SECTION("Map literals can be used as atoms")
    {
        auto result = parse("{foo: 1}[\"foo\"]");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 1_f);
    }

    SECTION("Map literals cannot be followed by postfix across newlines")
    {
        auto result =
            frst::parse_program(std::string{"{a: 1}\n[\"a\"]"});
        REQUIRE(result.has_value());
        CHECK(result.value().size() == 2);
    }

    SECTION("Bracketed identifier keys use the expression value")
    {
        auto result = parse("{[foo]: 1}");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        table.define("foo", frst::Value::create(42_f));
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Map>());
        const auto& map = out->raw_get<frst::Map>();
        auto key = frst::Value::create(42_f);
        REQUIRE(map.find(key) != map.end());
    }

    SECTION("Key expressions can be complex")
    {
        auto result = parse("{[if true: 1 else: 2]: 3}");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Map>());
        const auto& map = out->raw_get<frst::Map>();
        auto key = frst::Value::create(1_f);
        REQUIRE(map.find(key) != map.end());
    }

    SECTION("Value expressions can be complex")
    {
        auto result =
            parse("{a: if true: 1 else: 2, b: fn(x) -> { x }(3), c: [1,2]}");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Map>());
        const auto& map = out->raw_get<frst::Map>();
        auto a_key = frst::Value::create(std::string{"a"});
        auto b_key = frst::Value::create(std::string{"b"});
        auto c_key = frst::Value::create(std::string{"c"});
        REQUIRE(map.find(a_key) != map.end());
        REQUIRE(map.find(b_key) != map.end());
        REQUIRE(map.find(c_key) != map.end());
    }

    SECTION("Complex bracketed keys parse but enforce key type at eval")
    {
        auto result = parse("{[fn() -> { 1 }()]: 2, [[1,2]]: 3}");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        CHECK_THROWS_WITH(expr->evaluate(ctx),
                          Catch::Matchers::ContainsSubstring(
                              "Map keys may only be primitive values"));
    }

    SECTION("Nested map literals are allowed")
    {
        auto result = parse("{a: {b: 1}}");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Map>());
        const auto& map = out->raw_get<frst::Map>();
        auto key = frst::Value::create(std::string{"a"});
        auto it = map.find(key);
        REQUIRE(it != map.end());
        REQUIRE(it->second->is<frst::Map>());
    }

    SECTION("Map literals are valid in postfix and call contexts")
    {
        auto result = parse("{a: 1}[\"a\"]");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Int>());
        CHECK(out->get<frst::Int>().value() == 1_f);

        auto result2 = parse("({a: 1}).a");
        REQUIRE(result2.has_value());
        auto expr2 = std::move(result2).value();
        auto out2 = expr2->evaluate(ctx);
        REQUIRE(out2->is<frst::Int>());
        CHECK(out2->get<frst::Int>().value() == 1_f);

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

        auto callable = std::make_shared<IdentityCallable>();
        table.define("id", frst::Value::create(frst::Function{callable}));

        auto result3 = parse("id({a: 1})");
        REQUIRE(result3.has_value());
        auto expr3 = std::move(result3).value();
        auto out3 = expr3->evaluate(ctx);
        REQUIRE(out3->is<frst::Map>());
    }

    SECTION("Trailing comma with comments is accepted")
    {
        auto result = parse("{a: 1, # c\n}");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Map>());
        CHECK(out->raw_get<frst::Map>().size() == 1);
    }

    SECTION("Whitespace inside empty map is allowed")
    {
        auto result = parse("{ }");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();
        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto out = expr->evaluate(ctx);
        REQUIRE(out->is<frst::Map>());
        CHECK(out->raw_get<frst::Map>().empty());
    }

    SECTION("Malformed maps are rejected")
    {
        CHECK(not parse("{"));
        CHECK(not parse("{foo: 1"));
        CHECK(not parse("{,}"));
        CHECK(not parse("{foo:1,,bar:2}"));
        CHECK(not parse("{[1: 2]}"));
        CHECK(not parse("{[1]:}"));
        CHECK(not parse("%"));
        CHECK(not parse("%[]"));
        CHECK(not parse("{a: 1; b: 2}"));
        CHECK(not parse("{[1]: 2; [3]: 4}"));
        CHECK(not parse("{a: 1; }"));
        CHECK(not parse("{;a: 1}"));
    }

    SECTION("Identifier key sugar gets source range")
    {
        // "{foo: 1, bar: 2}"
        //   ^       ^
        //  col 2-4  col 10-12
        auto result = parse("{foo: 1, bar: 2}");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        auto nodes = expr->walk() | std::ranges::to<std::vector>();
        std::vector<const frst::ast::AST_Node*> key_literals;
        for (auto* n : nodes)
            if (n->node_label().starts_with("Literal(\""))
                key_literals.push_back(n);
        REQUIRE(key_literals.size() == 2);

        auto foo_range = key_literals[0]->source_range();
        CHECK(foo_range.begin.line == 1);
        CHECK(foo_range.begin.column == 2);
        CHECK(foo_range.end.column == 4);

        auto bar_range = key_literals[1]->source_range();
        CHECK(bar_range.begin.column == 10);
        CHECK(bar_range.end.column == 12);
    }

    SECTION("Identifier key range excludes surrounding whitespace")
    {
        // "{  foo  : 1}"
        //     ^
        //  col 4-6
        auto result = parse("{  foo  : 1}");
        REQUIRE(result.has_value());
        auto expr = std::move(result).value();

        auto nodes = expr->walk() | std::ranges::to<std::vector>();
        std::vector<const frst::ast::AST_Node*> key_literals;
        for (auto* n : nodes)
            if (n->node_label().starts_with("Literal(\""))
                key_literals.push_back(n);
        REQUIRE(key_literals.size() == 1);

        auto range = key_literals[0]->source_range();
        CHECK(range.begin.column == 4);
        CHECK(range.end.column == 6);
    }
}
