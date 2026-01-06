#include <catch2/catch_test_macros.hpp>

#include <catch2/trompeloeil.hpp>

#include <frost/mock/mock-expression.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/ast.hpp>
#include <frost/symbol-table.hpp>

using namespace frst;
using namespace std::literals;

using trompeloeil::_;

TEST_CASE("Array Constructor")
{
    constexpr auto make = mock::Mock_Expression::make;

    std::vector<mock::Mock_Expression::Ptr> elems;
    elems.emplace_back(make());

    Symbol_Table syms;

    SECTION("Empty")
    {
        ast::Array_Constructor node{{}};
        auto res = node.evaluate(syms);
        CHECK(res->get<Array>().value().empty());
    }

    // SECTION("One")
    // {
    //     REQUIRE_CALL(*elems[0], evaluate(_))
    //         .LR_WITH(&_1 == &syms)
    //         .RETURN(Value::create(42_f));

    //     ast::Array_Constructor node{{std::move(elems[0])}};
    //     auto res = node.evaluate(syms);
    // }
}
