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

            ast::Index node(std::move(struct_expr), std::move(idx_expr));

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
