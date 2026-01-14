#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>
#include <catch2/trompeloeil.hpp>

#include <frost/mock/mock-expression.hpp>
#include <frost/mock/mock-symbol-table.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/ast.hpp>
#include <frost/symbol-table.hpp>

using namespace frst;
using namespace std::literals;

using Catch::Matchers::ContainsSubstring;
using trompeloeil::_;

namespace
{
struct Recording_Mapper final : Callable
{
    mutable std::vector<std::vector<Value_Ptr>> calls;
    mutable std::size_t call_index = 0;
    std::vector<Value_Ptr> results;

    Value_Ptr call(const std::vector<Value_Ptr>& args) const override
    {
        calls.push_back(args);
        if (call_index < results.size())
            return results.at(call_index++);
        return Value::null();
    }

    std::string debug_dump() const override
    {
        return "<recording>";
    }
};

struct Throw_On_Index_Mapper final : Callable
{
    explicit Throw_On_Index_Mapper(std::size_t throw_on_index)
        : throw_on_index{throw_on_index}
    {
    }

    Value_Ptr call(const std::vector<Value_Ptr>& args) const override
    {
        calls.push_back(args);
        if (call_index++ == throw_on_index)
            throw Frost_Error{"kaboom"};
        return Value::null();
    }

    std::string debug_dump() const override
    {
        return "<throw-on-index>";
    }

    std::size_t throw_on_index;
    mutable std::size_t call_index = 0;
    mutable std::vector<std::vector<Value_Ptr>> calls;
};
} // namespace

TEST_CASE("Map Array")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    mock::Mock_Symbol_Table syms;
    auto structure_expr = mock::Mock_Expression::make();
    auto operation_expr = mock::Mock_Expression::make();

    SECTION("Success cases")
    {
        SECTION("Empty array returns empty array; op not called")
        {
            auto empty = Value::create(Array{});
            auto mapper = std::make_shared<Recording_Mapper>();
            auto op_val = Value::create(Function{mapper});

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(empty);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);

            ast::Map node{std::move(structure_expr), std::move(operation_expr)};

            auto res = node.evaluate(syms);
            auto out = res->get<Array>().value();
            CHECK(out.empty());
            CHECK(mapper->calls.empty());
        }

        SECTION("Maps each element in order with pointer-equal arguments")
        {
            auto v1 = Value::create(1_f);
            auto v2 = Value::create(2_f);
            auto v3 = Value::create(3_f);
            auto array_val = Value::create(Array{v1, v2, v3});

            auto mapper = std::make_shared<Recording_Mapper>();
            auto r1 = Value::create("one"s);
            auto r2 = Value::create("two"s);
            auto r3 = Value::create("three"s);
            mapper->results = {r1, r2, r3};
            auto op_val = Value::create(Function{mapper});

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(array_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);

            ast::Map node{std::move(structure_expr), std::move(operation_expr)};

            auto res = node.evaluate(syms);
            auto out = res->get<Array>().value();
            REQUIRE(out.size() == 3);
            CHECK(out.at(0) == r1);
            CHECK(out.at(1) == r2);
            CHECK(out.at(2) == r3);

            REQUIRE(mapper->calls.size() == 3);
            CHECK(mapper->calls.at(0).at(0) == v1);
            CHECK(mapper->calls.at(1).at(0) == v2);
            CHECK(mapper->calls.at(2).at(0) == v3);
        }

        SECTION("Propagates mapper error and stops")
        {
            auto v1 = Value::create(1_f);
            auto v2 = Value::create(2_f);
            auto v3 = Value::create(3_f);
            auto array_val = Value::create(Array{v1, v2, v3});

            auto mapper = std::make_shared<Throw_On_Index_Mapper>(1);
            auto op_val = Value::create(Function{mapper});

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(array_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);

            ast::Map node{std::move(structure_expr), std::move(operation_expr)};

            CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("kaboom"));
            REQUIRE(mapper->calls.size() == 2);
            CHECK(mapper->calls.at(0).at(0) == v1);
            CHECK(mapper->calls.at(1).at(0) == v2);
        }
    }

    SECTION("Error cases")
    {
        SECTION("Non-structured input errors and skips operation evaluation")
        {
            auto not_structured = Value::create(42_f);
            auto op_val = Value::create("nope"s);

            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .RETURN(not_structured);
            FORBID_CALL(*operation_expr, evaluate(_));

            ast::Map node{std::move(structure_expr), std::move(operation_expr)};

            CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("Int"));
        }

        SECTION("Non-function operation errors and includes type name")
        {
            auto array_val = Value::create(Array{});
            auto op_val = Value::create("not-a-function"s);

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(array_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);

            ast::Map node{std::move(structure_expr), std::move(operation_expr)};

            CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("String"));
        }
    }
}

TEST_CASE("Map Map")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    mock::Mock_Symbol_Table syms;
    auto structure_expr = mock::Mock_Expression::make();
    auto operation_expr = mock::Mock_Expression::make();

    SECTION("Success cases")
    {
        SECTION("Empty map returns empty map; op not called")
        {
            auto empty = Value::create(Map{});
            auto mapper = std::make_shared<Recording_Mapper>();
            auto op_val = Value::create(Function{mapper});

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(empty);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);

            ast::Map node{std::move(structure_expr), std::move(operation_expr)};

            auto res = node.evaluate(syms);
            auto out = res->get<Map>().value();
            CHECK(out.empty());
            CHECK(mapper->calls.empty());
        }

        SECTION("Calls mapper with (key, value) and merges results")
        {
            auto k1 = Value::create("k1"s);
            auto v1 = Value::create(1_f);
            auto k2 = Value::create("k2"s);
            auto v2 = Value::create(2_f);

            auto input_map = Value::create(Map{{k1, v1}, {k2, v2}});

            auto out_k1 = Value::create("a"s);
            auto out_v1 = Value::create(10_f);
            auto out_k2 = Value::create("b"s);
            auto out_v2 = Value::create(20_f);

            auto mapper = std::make_shared<Recording_Mapper>();
            mapper->results = {
                Value::create(Map{{out_k1, out_v1}}),
                Value::create(Map{{out_k2, out_v2}}),
            };
            auto op_val = Value::create(Function{mapper});

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(input_map);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);

            ast::Map node{std::move(structure_expr), std::move(operation_expr)};

            auto res = node.evaluate(syms);
            auto out = res->get<Map>().value();
            REQUIRE(out.size() == 2);
            CHECK(out.at(out_k1) == out_v1);
            CHECK(out.at(out_k2) == out_v2);

            REQUIRE(mapper->calls.size() == 2);
            std::vector<std::pair<Value_Ptr, Value_Ptr>> expected = {{k1, v1},
                                                                     {k2, v2}};
            for (const auto& call : mapper->calls)
            {
                REQUIRE(call.size() == 2);
                bool matched = false;
                for (auto it = expected.begin(); it != expected.end(); ++it)
                {
                    if (it->first == call.at(0) && it->second == call.at(1))
                    {
                        expected.erase(it);
                        matched = true;
                        break;
                    }
                }
                CHECK(matched);
            }
            CHECK(expected.empty());
        }

        SECTION("Mapper results with multiple entries are merged")
        {
            auto k1 = Value::create("k1"s);
            auto v1 = Value::create(1_f);
            auto k2 = Value::create("k2"s);
            auto v2 = Value::create(2_f);

            auto input_map = Value::create(Map{{k1, v1}, {k2, v2}});

            auto out_k1 = Value::create("a"s);
            auto out_v1 = Value::create(10_f);
            auto out_k2 = Value::create("b"s);
            auto out_v2 = Value::create(20_f);
            auto out_k3 = Value::create("c"s);
            auto out_v3 = Value::create(30_f);
            auto out_k4 = Value::create("d"s);
            auto out_v4 = Value::create(40_f);

            auto mapper = std::make_shared<Recording_Mapper>();
            mapper->results = {
                Value::create(Map{{out_k1, out_v1}, {out_k2, out_v2}}),
                Value::create(Map{{out_k3, out_v3}, {out_k4, out_v4}}),
            };
            auto op_val = Value::create(Function{mapper});

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(input_map);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);

            ast::Map node{std::move(structure_expr), std::move(operation_expr)};

            auto res = node.evaluate(syms);
            auto out = res->get<Map>().value();
            REQUIRE(out.size() == 4);
            CHECK(out.at(out_k1) == out_v1);
            CHECK(out.at(out_k2) == out_v2);
            CHECK(out.at(out_k3) == out_v3);
            CHECK(out.at(out_k4) == out_v4);
        }
    }

    SECTION("Error cases")
    {
        SECTION("Mapper return must be a map")
        {
            auto k1 = Value::create("k1"s);
            auto v1 = Value::create(1_f);
            auto input_map = Value::create(Map{{k1, v1}});

            auto mapper = std::make_shared<Recording_Mapper>();
            mapper->results = {Value::create(123_f)};
            auto op_val = Value::create(Function{mapper});

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(input_map);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);

            ast::Map node{std::move(structure_expr), std::move(operation_expr)};

            CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("Int"));
            CHECK(mapper->calls.size() == 1);
        }

        SECTION("Mapper errors propagate")
        {
            auto k1 = Value::create("k1"s);
            auto v1 = Value::create(1_f);
            auto k2 = Value::create("k2"s);
            auto v2 = Value::create(2_f);
            auto input_map = Value::create(Map{{k1, v1}, {k2, v2}});

            auto mapper = std::make_shared<Throw_On_Index_Mapper>(0);
            auto op_val = Value::create(Function{mapper});

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(input_map);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);

            ast::Map node{std::move(structure_expr), std::move(operation_expr)};

            CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("kaboom"));
            CHECK(mapper->calls.size() == 1);
        }
    }
}
