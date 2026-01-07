#include <catch2/catch_test_macros.hpp>

#include <catch2/trompeloeil.hpp>

#include <frost/mock/mock-expression.hpp>
#include <frost/mock/mock-symbol-table.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/ast.hpp>

using namespace frst;
using namespace std::literals;

using trompeloeil::_;

TEST_CASE("Array Index")
{
    auto values =
        Array{Value::create(10_f), Value::create(20_f), Value::create(30_f)};
    auto arr = Value::create(auto{values});

    for (auto index : std::views::iota(-5, 6))
        DYNAMIC_SECTION("Index " << index)
        {
            auto struct_expr = mock::Mock_Expression::make();
            auto idx_expr = mock::Mock_Expression::make();
            auto syms = mock::Mock_Symbol_Table{};
            trompeloeil::sequence seq;

            REQUIRE_CALL(*struct_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(arr);

            REQUIRE_CALL(*idx_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(Value::create(Int{index}));

            ast::Index node{std::move(struct_expr), std::move(idx_expr)};

            auto res = node.evaluate(syms);

            Value_Ptr expect = Value::create(Null{});
            switch (index)
            {
            case 0:
                [[fallthrough]];
            case -3:
                expect = values.at(0);
                break;
            case 1:
                [[fallthrough]];
            case -2:
                expect = values.at(1);
                break;
            case 2:
                [[fallthrough]];
            case -1:
                expect = values.at(2);
                break;
            }

            if (expect->is<Null>())
                CHECK(res->is<Null>());
            else
                CHECK(res == expect);
        }
}

TEST_CASE("Map Index Primitive Key")
{
    // AI-generated test additions by Codex (GPT-5).
    auto key_null = Value::create();
    auto key_int = Value::create(1_f);
    auto key_float = Value::create(1.0);
    auto key_bool = Value::create(true);
    auto key_string = Value::create("hello"s);

    auto val_null = Value::create("null-key"s);
    auto val_int = Value::create("int-key"s);
    auto val_float = Value::create("float-key"s);
    auto val_bool = Value::create("bool-key"s);
    auto val_string = Value::create("string-key"s);

    auto map = Value::create(Map{
        {key_null, val_null},
        {key_int, val_int},
        {key_float, val_float},
        {key_bool, val_bool},
        {key_string, val_string},
    });

    struct Case
    {
        const char* name;
        Value_Ptr query;
        Value_Ptr expected;
    };

    std::vector<Case> cases = {
        {"null", Value::create(), val_null},
        {"int", Value::create(1_f), val_int},
        {"float", Value::create(1.0), val_float},
        {"bool", Value::create(true), val_bool},
        {"string", Value::create("hello"s), val_string},
        {"missing_int", Value::create(2_f), Value::create(Null{})},
        {"missing_float", Value::create(2.0), Value::create(Null{})},
        {"missing_string", Value::create("nope"s), Value::create(Null{})},
    };

    for (const auto& test : cases)
    {
        DYNAMIC_SECTION("Primitive key: " << test.name)
        {
            auto struct_expr = mock::Mock_Expression::make();
            auto idx_expr = mock::Mock_Expression::make();
            auto syms = mock::Mock_Symbol_Table{};
            trompeloeil::sequence seq;

            REQUIRE_CALL(*struct_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(map);

            REQUIRE_CALL(*idx_expr, evaluate(_))
                .LR_WITH(&_1 == &syms)
                .IN_SEQUENCE(seq)
                .RETURN(test.query);

            ast::Index node{std::move(struct_expr), std::move(idx_expr)};

            auto res = node.evaluate(syms);

            if (test.expected->is<Null>())
                CHECK(res->is<Null>());
            else
                CHECK(res == test.expected);
        }
    }
}

TEST_CASE("Map Index Structure Key")
{
}
