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
struct Recording_Foreach final : Callable
{
    mutable std::vector<std::vector<Value_Ptr>> calls;

    Value_Ptr call(const std::vector<Value_Ptr>& args) const override
    {
        calls.push_back(args);
        return Value::null();
    }

    std::string debug_dump() const override
    {
        return "<recording>";
    }
};

struct Throw_On_Call final : Callable
{
    explicit Throw_On_Call(std::size_t throw_on_index)
        : throw_on_index{throw_on_index}
    {
    }

    Value_Ptr call(const std::vector<Value_Ptr>& args) const override
    {
        calls.push_back(args);
        if (call_index++ == throw_on_index)
            throw Frost_User_Error{"kaboom"};
        return Value::null();
    }

    std::string debug_dump() const override
    {
        return "<throw-on-call>";
    }

    std::size_t throw_on_index;
    mutable std::size_t call_index = 0;
    mutable std::vector<std::vector<Value_Ptr>> calls;
};
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
            auto op = std::make_shared<Recording_Foreach>();
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

            ast::Foreach node{std::move(structure_expr),
                              std::move(operation_expr)};

            auto res = node.evaluate(syms);
            CHECK(res->is<Null>());
            CHECK(op->calls.empty());
        }

        SECTION("Calls op for each element in order with pointer-equal args")
        {
            auto v1 = Value::create(1_f);
            auto v2 = Value::create(2_f);
            auto v3 = Value::create(3_f);
            auto array_val = Value::create(Array{v1, v2, v3});

            auto op = std::make_shared<Recording_Foreach>();
            auto op_val = Value::create(Function{op});

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

            auto res = node.evaluate(syms);
            CHECK(res->is<Null>());
            REQUIRE(op->calls.size() == 3);
            REQUIRE(op->calls.at(0).size() == 1);
            REQUIRE(op->calls.at(1).size() == 1);
            REQUIRE(op->calls.at(2).size() == 1);
            CHECK(op->calls.at(0).at(0) == v1);
            CHECK(op->calls.at(1).at(0) == v2);
            CHECK(op->calls.at(2).at(0) == v3);
        }

        SECTION("Op error propagates and stops iteration")
        {
            auto v1 = Value::create(1_f);
            auto v2 = Value::create(2_f);
            auto v3 = Value::create(3_f);
            auto array_val = Value::create(Array{v1, v2, v3});

            auto op = std::make_shared<Throw_On_Call>(1);
            auto op_val = Value::create(Function{op});

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

            CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("kaboom"));
            REQUIRE(op->calls.size() == 2);
            REQUIRE(op->calls.at(0).size() == 1);
            REQUIRE(op->calls.at(1).size() == 1);
            CHECK(op->calls.at(0).at(0) == v1);
            CHECK(op->calls.at(1).at(0) == v2);
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
            auto op = std::make_shared<Recording_Foreach>();
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

            ast::Foreach node{std::move(structure_expr),
                              std::move(operation_expr)};

            auto res = node.evaluate(syms);
            CHECK(res->is<Null>());
            CHECK(op->calls.empty());
        }

        SECTION("Calls op for each entry with pointer-equal args")
        {
            auto k1 = Value::create("k1"s);
            auto v1 = Value::create(1_f);
            auto k2 = Value::create("k2"s);
            auto v2 = Value::create(2_f);
            auto map_val = Value::create(Map{{k1, v1}, {k2, v2}});

            auto op = std::make_shared<Recording_Foreach>();
            auto op_val = Value::create(Function{op});

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

            auto res = node.evaluate(syms);
            CHECK(res->is<Null>());
            REQUIRE(op->calls.size() == 2);
            for (const auto& call : op->calls)
            {
                REQUIRE(call.size() == 2);
            }

            std::vector<std::pair<Value_Ptr, Value_Ptr>> expected = {{k1, v1},
                                                                     {k2, v2}};
            for (const auto& call : op->calls)
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

            auto op = std::make_shared<Throw_On_Call>(0);
            auto op_val = Value::create(Function{op});

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

            CHECK_THROWS_WITH(node.evaluate(syms), ContainsSubstring("kaboom"));
            REQUIRE(op->calls.size() == 1);
            REQUIRE(op->calls.at(0).size() == 2);
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
