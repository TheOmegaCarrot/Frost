#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>
#include <catch2/trompeloeil.hpp>

#include <frost/mock/mock-callable.hpp>
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
using Call_List = std::vector<std::vector<Value_Ptr>>;

void record_call(Call_List& calls, std::span<const Value_Ptr> args)
{
    calls.emplace_back(args.begin(), args.end());
}
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
            auto mapper = mock::Mock_Callable::make();
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
            FORBID_CALL(*mapper, call(_));

            ast::Map node{std::move(structure_expr), std::move(operation_expr)};

            auto res = node.evaluate(syms);
            auto out = res->get<Array>().value();
            CHECK(out.empty());
        }

        SECTION("Maps each element in order with pointer-equal arguments")
        {
            auto v1 = Value::create(1_f);
            auto v2 = Value::create(2_f);
            auto v3 = Value::create(3_f);
            auto array_val = Value::create(Array{v1, v2, v3});

            auto mapper = mock::Mock_Callable::make();
            auto r1 = Value::create("one"s);
            auto r2 = Value::create("two"s);
            auto r3 = Value::create("three"s);
            auto op_val = Value::create(Function{mapper});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(array_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            REQUIRE_CALL(*mapper, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .IN_SEQUENCE(seq)
                .RETURN(r1);
            REQUIRE_CALL(*mapper, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .IN_SEQUENCE(seq)
                .RETURN(r2);
            REQUIRE_CALL(*mapper, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .IN_SEQUENCE(seq)
                .RETURN(r3);

            ast::Map node{std::move(structure_expr), std::move(operation_expr)};

            auto res = node.evaluate(syms);
            auto out = res->get<Array>().value();
            REQUIRE(out.size() == 3);
            CHECK(out.at(0) == r1);
            CHECK(out.at(1) == r2);
            CHECK(out.at(2) == r3);

            REQUIRE(calls.size() == 3);
            CHECK(calls.at(0).at(0) == v1);
            CHECK(calls.at(1).at(0) == v2);
            CHECK(calls.at(2).at(0) == v3);
        }

        SECTION("Propagates mapper error and stops")
        {
            auto v1 = Value::create(1_f);
            auto v2 = Value::create(2_f);
            auto v3 = Value::create(3_f);
            auto array_val = Value::create(Array{v1, v2, v3});

            auto mapper = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{mapper});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(array_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            REQUIRE_CALL(*mapper, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .IN_SEQUENCE(seq)
                .RETURN(Value::null());
            REQUIRE_CALL(*mapper, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .IN_SEQUENCE(seq)
                .THROW(Frost_User_Error{"kaboom"});

            ast::Map node{std::move(structure_expr), std::move(operation_expr)};

            CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("kaboom"));
            REQUIRE(calls.size() == 2);
            CHECK(calls.at(0).at(0) == v1);
            CHECK(calls.at(1).at(0) == v2);
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
            auto mapper = mock::Mock_Callable::make();
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
            FORBID_CALL(*mapper, call(_));

            ast::Map node{std::move(structure_expr), std::move(operation_expr)};

            auto res = node.evaluate(syms);
            auto out = res->get<Map>().value();
            CHECK(out.empty());
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

            auto mapper = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{mapper});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(input_map);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            REQUIRE_CALL(*mapper, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .RETURN(Value::create(Map{{out_k1, out_v1}}));
            REQUIRE_CALL(*mapper, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .RETURN(Value::create(Map{{out_k2, out_v2}}));

            ast::Map node{std::move(structure_expr), std::move(operation_expr)};

            auto res = node.evaluate(syms);
            auto out = res->get<Map>().value();
            REQUIRE(out.size() == 2);
            CHECK(out.at(out_k1) == out_v1);
            CHECK(out.at(out_k2) == out_v2);

            REQUIRE(calls.size() == 2);
            std::vector<std::pair<Value_Ptr, Value_Ptr>> expected = {{k1, v1},
                                                                     {k2, v2}};
            for (const auto& call : calls)
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

            auto mapper = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{mapper});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(input_map);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            REQUIRE_CALL(*mapper, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .RETURN(Value::create(Map{{out_k1, out_v1}, {out_k2, out_v2}}));
            REQUIRE_CALL(*mapper, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .RETURN(Value::create(Map{{out_k3, out_v3}, {out_k4, out_v4}}));

            ast::Map node{std::move(structure_expr), std::move(operation_expr)};

            auto res = node.evaluate(syms);
            auto out = res->get<Map>().value();
            REQUIRE(out.size() == 4);
            CHECK(out.at(out_k1) == out_v1);
            CHECK(out.at(out_k2) == out_v2);
            CHECK(out.at(out_k3) == out_v3);
            CHECK(out.at(out_k4) == out_v4);
        }

        SECTION("Mapper can return an empty map for an entry")
        {
            auto k1 = Value::create("k1"s);
            auto v1 = Value::create(1_f);
            auto k2 = Value::create("k2"s);
            auto v2 = Value::create(2_f);

            auto input_map = Value::create(Map{{k1, v1}, {k2, v2}});

            auto out_k1 = Value::create("a"s);
            auto out_v1 = Value::create(10_f);

            auto mapper = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{mapper});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(input_map);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            REQUIRE_CALL(*mapper, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .RETURN(Value::create(Map{{out_k1, out_v1}}));
            REQUIRE_CALL(*mapper, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .RETURN(Value::create(Map{}));

            ast::Map node{std::move(structure_expr), std::move(operation_expr)};

            auto res = node.evaluate(syms);
            auto out = res->get<Map>().value();
            REQUIRE(out.size() == 1);
            CHECK(out.at(out_k1) == out_v1);
        }
    }

    SECTION("Error cases")
    {
        SECTION("Key collision errors")
        {
            auto k1 = Value::create("k1"s);
            auto v1 = Value::create(1_f);
            auto k2 = Value::create("k2"s);
            auto v2 = Value::create(2_f);
            auto input_map = Value::create(Map{{k1, v1}, {k2, v2}});

            auto dup_key = Value::create("dup"s);
            auto mapper = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{mapper});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(input_map);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            REQUIRE_CALL(*mapper, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .RETURN(Value::create(Map{{dup_key, Value::create(10_f)}}));
            REQUIRE_CALL(*mapper, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .RETURN(Value::create(Map{{dup_key, Value::create(20_f)}}));

            ast::Map node{std::move(structure_expr), std::move(operation_expr)};

            CHECK_THROWS_WITH(node.evaluate(syms),
                              ContainsSubstring("collision"));
            REQUIRE(calls.size() == 2);
        }

        SECTION("Collision on value-equal primitive keys with distinct ptrs")
        {
            auto k1 = Value::create("k1"s);
            auto v1 = Value::create(1_f);
            auto k2 = Value::create("k2"s);
            auto v2 = Value::create(2_f);
            auto input_map = Value::create(Map{{k1, v1}, {k2, v2}});

            auto dup_key1 = Value::create(99_f);
            auto dup_key2 = Value::create(99_f);

            auto mapper = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{mapper});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(input_map);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            REQUIRE_CALL(*mapper, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .RETURN(Value::create(Map{{dup_key1, Value::create(10_f)}}));
            REQUIRE_CALL(*mapper, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .RETURN(Value::create(Map{{dup_key2, Value::create(20_f)}}));

            ast::Map node{std::move(structure_expr), std::move(operation_expr)};

            CHECK_THROWS_WITH(node.evaluate(syms),
                              ContainsSubstring("collision"));
            REQUIRE(calls.size() == 2);
        }

        SECTION("Collision on structured keys uses identity semantics")
        {
            auto k1 = Value::create("k1"s);
            auto v1 = Value::create(1_f);
            auto k2 = Value::create("k2"s);
            auto v2 = Value::create(2_f);
            auto input_map = Value::create(Map{{k1, v1}, {k2, v2}});

            auto shared_key =
                Value::create(Map{{Value::create(1_f), Value::create(2_f)}});

            auto mapper = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{mapper});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(input_map);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            REQUIRE_CALL(*mapper, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .RETURN(Value::create(Map{{shared_key, Value::create(10_f)}}));
            REQUIRE_CALL(*mapper, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .RETURN(Value::create(Map{{shared_key, Value::create(20_f)}}));

            ast::Map node{std::move(structure_expr), std::move(operation_expr)};

            CHECK_THROWS_WITH(node.evaluate(syms),
                              ContainsSubstring("collision"));
            REQUIRE(calls.size() == 2);
        }

        SECTION("Mapper return must be a map")
        {
            auto k1 = Value::create("k1"s);
            auto v1 = Value::create(1_f);
            auto input_map = Value::create(Map{{k1, v1}});

            auto mapper = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{mapper});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(input_map);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            REQUIRE_CALL(*mapper, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .RETURN(Value::create(123_f));

            ast::Map node{std::move(structure_expr), std::move(operation_expr)};

            CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("Int"));
            CHECK(calls.size() == 1);
        }

        SECTION("Mapper errors propagate")
        {
            auto k1 = Value::create("k1"s);
            auto v1 = Value::create(1_f);
            auto k2 = Value::create("k2"s);
            auto v2 = Value::create(2_f);
            auto input_map = Value::create(Map{{k1, v1}, {k2, v2}});

            auto mapper = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{mapper});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(input_map);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            REQUIRE_CALL(*mapper, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .THROW(Frost_User_Error{"kaboom"});

            ast::Map node{std::move(structure_expr), std::move(operation_expr)};

            CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("kaboom"));
            CHECK(calls.size() == 1);
        }
    }
}
