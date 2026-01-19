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

TEST_CASE("Foreach Array")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    mock::Mock_Symbol_Table syms;
    auto structure_expr = mock::Mock_Expression::make();
    auto operation_expr = mock::Mock_Expression::make();

    SECTION("Success cases")
    {
        SECTION("Empty array returns null; op not called")
        {
            auto empty = Value::create(Array{});
            auto op = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{op});

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(empty);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            FORBID_CALL(*op, call(_));

            ast::Foreach node{std::move(structure_expr),
                              std::move(operation_expr)};

            auto res = node.evaluate(syms);
            CHECK(res->is<Null>());
        }

        SECTION("Calls op for each element in order with pointer-equal args")
        {
            auto v1 = Value::create(1_f);
            auto v2 = Value::create(2_f);
            auto v3 = Value::create(3_f);
            auto array_val = Value::create(Array{v1, v2, v3});

            auto op = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{op});
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
            REQUIRE_CALL(*op, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .RETURN(Value::create(true))
                .TIMES(3);

            ast::Foreach node{std::move(structure_expr),
                              std::move(operation_expr)};

            auto res = node.evaluate(syms);
            CHECK(res->is<Null>());
            REQUIRE(calls.size() == 3);
            REQUIRE(calls.at(0).size() == 1);
            REQUIRE(calls.at(1).size() == 1);
            REQUIRE(calls.at(2).size() == 1);
            CHECK(calls.at(0).at(0) == v1);
            CHECK(calls.at(1).at(0) == v2);
            CHECK(calls.at(2).at(0) == v3);
        }

        SECTION("Op error propagates and stops iteration")
        {
            auto v1 = Value::create(1_f);
            auto v2 = Value::create(2_f);
            auto v3 = Value::create(3_f);
            auto array_val = Value::create(Array{v1, v2, v3});

            auto op = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{op});
            Call_List calls;
            std::size_t call_index = 0;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(array_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            REQUIRE_CALL(*op, call(_))
                .LR_SIDE_EFFECT({
                    record_call(calls, _1);
                    if (call_index++ == 1)
                        throw Frost_User_Error{"kaboom"};
                })
                .RETURN(Value::create(true))
                .TIMES(2);

            ast::Foreach node{std::move(structure_expr),
                              std::move(operation_expr)};

            CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("kaboom"));
            REQUIRE(calls.size() == 2);
            REQUIRE(calls.at(0).size() == 1);
            REQUIRE(calls.at(1).size() == 1);
            CHECK(calls.at(0).at(0) == v1);
            CHECK(calls.at(1).at(0) == v2);
        }

        SECTION("Falsy op return stops iteration")
        {
            auto v1 = Value::create(1_f);
            auto v2 = Value::create(2_f);
            auto v3 = Value::create(3_f);
            auto array_val = Value::create(Array{v1, v2, v3});

            auto op = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{op});
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
            REQUIRE_CALL(*op, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .RETURN(Value::create(false))
                .TIMES(1);

            ast::Foreach node{std::move(structure_expr),
                              std::move(operation_expr)};

            auto res = node.evaluate(syms);
            CHECK(res->is<Null>());
            REQUIRE(calls.size() == 1);
            REQUIRE(calls.at(0).size() == 1);
            CHECK(calls.at(0).at(0) == v1);
        }
    }

    SECTION("Error cases")
    {
        SECTION("Non-structured value errors and skips op evaluation")
        {
            auto bad_val = Value::create(123_f);

            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .RETURN(bad_val);
            FORBID_CALL(*operation_expr, evaluate(_));

            ast::Foreach node{std::move(structure_expr),
                              std::move(operation_expr)};

            CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("Int"));
        }

        SECTION("Non-function operation errors and includes type name")
        {
            auto array_val = Value::create(Array{});
            auto op_val = Value::create(123_f);

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(array_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);

            ast::Foreach node{std::move(structure_expr),
                              std::move(operation_expr)};

            CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("Int"));
        }
    }
}

TEST_CASE("Foreach Map")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    mock::Mock_Symbol_Table syms;
    auto structure_expr = mock::Mock_Expression::make();
    auto operation_expr = mock::Mock_Expression::make();

    SECTION("Success cases")
    {
        SECTION("Empty map returns null; op not called")
        {
            auto empty = Value::create(Map{});
            auto op = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{op});

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(empty);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            FORBID_CALL(*op, call(_));

            ast::Foreach node{std::move(structure_expr),
                              std::move(operation_expr)};

            auto res = node.evaluate(syms);
            CHECK(res->is<Null>());
        }

        SECTION("Calls op for each entry with pointer-equal args")
        {
            auto k1 = Value::create("k1"s);
            auto v1 = Value::create(1_f);
            auto k2 = Value::create("k2"s);
            auto v2 = Value::create(2_f);
            auto map_val = Value::create(Map{{k1, v1}, {k2, v2}});

            auto op = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{op});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(map_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            REQUIRE_CALL(*op, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .RETURN(Value::create(true))
                .TIMES(2);

            ast::Foreach node{std::move(structure_expr),
                              std::move(operation_expr)};

            auto res = node.evaluate(syms);
            CHECK(res->is<Null>());
            REQUIRE(calls.size() == 2);
            for (const auto& call : calls)
            {
                REQUIRE(call.size() == 2);
            }

            std::vector<std::pair<Value_Ptr, Value_Ptr>> expected = {{k1, v1},
                                                                     {k2, v2}};
            for (const auto& call : calls)
            {
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

        SECTION("Op error propagates and stops iteration")
        {
            auto k1 = Value::create("k1"s);
            auto v1 = Value::create(1_f);
            auto k2 = Value::create("k2"s);
            auto v2 = Value::create(2_f);
            auto map_val = Value::create(Map{{k1, v1}, {k2, v2}});

            auto op = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{op});
            Call_List calls;
            std::size_t call_index = 0;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(map_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            REQUIRE_CALL(*op, call(_))
                .LR_SIDE_EFFECT({
                    record_call(calls, _1);
                    if (call_index++ == 0)
                        throw Frost_User_Error{"kaboom"};
                })
                .RETURN(Value::create(true))
                .TIMES(1);

            ast::Foreach node{std::move(structure_expr),
                              std::move(operation_expr)};

            CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("kaboom"));
            REQUIRE(calls.size() == 1);
            REQUIRE(calls.at(0).size() == 2);
        }

        SECTION("Falsy op return stops iteration")
        {
            auto k1 = Value::create("k1"s);
            auto v1 = Value::create(1_f);
            auto k2 = Value::create("k2"s);
            auto v2 = Value::create(2_f);
            auto map_val = Value::create(Map{{k1, v1}, {k2, v2}});

            auto op = mock::Mock_Callable::make();
            auto op_val = Value::create(Function{op});
            Call_List calls;

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(map_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);
            REQUIRE_CALL(*op, call(_))
                .LR_SIDE_EFFECT(record_call(calls, _1))
                .RETURN(Value::create(false))
                .TIMES(1);

            ast::Foreach node{std::move(structure_expr),
                              std::move(operation_expr)};

            auto res = node.evaluate(syms);
            CHECK(res->is<Null>());
            REQUIRE(calls.size() == 1);
            REQUIRE(calls.at(0).size() == 2);

            std::vector<std::pair<Value_Ptr, Value_Ptr>> expected = {{k1, v1},
                                                                     {k2, v2}};
            bool matched = false;
            for (auto it = expected.begin(); it != expected.end(); ++it)
            {
                if (it->first
                    == calls.at(0).at(0)
                    && it->second
                    == calls.at(0).at(1))
                {
                    matched = true;
                    break;
                }
            }
            CHECK(matched);
        }
    }

    SECTION("Error cases")
    {
        SECTION("Non-structured value errors and skips op evaluation")
        {
            auto bad_val = Value::create(123_f);

            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .RETURN(bad_val);
            FORBID_CALL(*operation_expr, evaluate(_));

            ast::Foreach node{std::move(structure_expr),
                              std::move(operation_expr)};

            CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("Int"));
        }

        SECTION("Non-function operation errors and includes type name")
        {
            auto map_val = Value::create(Map{});
            auto op_val = Value::create("nope"s);

            trompeloeil::sequence seq;
            REQUIRE_CALL(*structure_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(map_val);
            REQUIRE_CALL(*operation_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(op_val);

            ast::Foreach node{std::move(structure_expr),
                              std::move(operation_expr)};

            CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("String"));
        }
    }
}
