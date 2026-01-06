#include <catch2/catch_test_macros.hpp>

#include <catch2/trompeloeil.hpp>

#include <frost/mock/mock-expression.hpp>
#include <frost/mock/mock-symbol-table.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/ast.hpp>

using namespace frst;
using namespace std::literals;

using trompeloeil::_;

TEST_CASE("Array Constructor")
{
    constexpr auto make = mock::Mock_Expression::make;

    auto e1 = make();
    auto e2 = make();
    auto e3 = make();
    auto e4 = make();

    auto val = Value::create(42_f);
    std::vector values{val, Value::create("hello"s), Value::create(true),
                       Value::create(Array{val})};

    mock::Mock_Symbol_Table syms;

    SECTION("Empty")
    {
        ast::Array_Constructor node{{}};
        auto res = node.evaluate(syms);
        CHECK(res->get<Array>().value().empty());
    }

    SECTION("One")
    {
        REQUIRE_CALL(*e1, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .RETURN(values.at(0));

        std::vector<ast::Expression::Ptr> v;
        v.emplace_back(std::move(e1));
        ast::Array_Constructor node{std::move(v)};
        auto res = node.evaluate(syms);
        CHECK(res->get<Array>()->size() == 1);
        CHECK(res->get<Array>()->front() == val);
    }

    SECTION("FOur")
    {
        REQUIRE_CALL(*e1, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .RETURN(values.at(0));
        REQUIRE_CALL(*e2, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .RETURN(values.at(1));
        REQUIRE_CALL(*e3, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .RETURN(values.at(2));
        REQUIRE_CALL(*e4, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .RETURN(values.at(3));

        // I love that `initializer_list`s are backed by a `const` array!
        // I just _love_ it!        :(
        std::vector<ast::Expression::Ptr> v;
        v.emplace_back(std::move(e1));
        v.emplace_back(std::move(e2));
        v.emplace_back(std::move(e3));
        v.emplace_back(std::move(e4));
        ast::Array_Constructor node{std::move(v)};
        auto res = node.evaluate(syms);
        CHECK(res->get<Array>()->size() == 4);
        CHECK(res->get<Array>().value() == values);
    }
}
