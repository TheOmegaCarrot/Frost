#include <catch2/catch_test_macros.hpp>

#include <catch2/trompeloeil.hpp>

#include <frost/mock/mock-expression.hpp>
#include <frost/mock/mock-symbol-table.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/ast.hpp>

using namespace frst;
using namespace std::literals;

using trompeloeil::_;

TEST_CASE("Map Constructor")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    constexpr auto make = mock::Mock_Expression::make;
    mock::Mock_Symbol_Table syms;

    SECTION("Empty")
    {
        ast::Map_Constructor node{{}};
        auto res = node.evaluate(syms);
        CHECK(res->get<Map>()->empty());
    }

    SECTION("One")
    {
        auto k1 = make();
        auto v1 = make();

        auto key = Value::create(1_f);
        auto val = Value::create("one"s);

        trompeloeil::sequence seq;

        REQUIRE_CALL(*k1, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(key);
        REQUIRE_CALL(*v1, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(val);

        std::vector<ast::Map_Constructor::KV_Pair> pairs;
        pairs.emplace_back(std::move(k1), std::move(v1));
        ast::Map_Constructor node{std::move(pairs)};

        auto res = node.evaluate(syms);
        auto map = res->get<Map>().value();
        CHECK(map.size() == 1);
        CHECK(map.at(key) == val);
        CHECK(map.at(Value::create(1_f)) == val);
    }

    SECTION("Multiple pairs preserve order")
    {
        auto k1 = make();
        auto v1 = make();
        auto k2 = make();
        auto v2 = make();
        auto k3 = make();
        auto v3 = make();

        auto key1 = Value::create(1_f);
        auto key2 = Value::create(2_f);
        auto key3 = Value::create(3_f);

        auto val1 = Value::create("one"s);
        auto val2 = Value::create("two"s);
        auto val3 = Value::create("three"s);

        trompeloeil::sequence seq;

        REQUIRE_CALL(*k1, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(key1);
        REQUIRE_CALL(*v1, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(val1);
        REQUIRE_CALL(*k2, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(key2);
        REQUIRE_CALL(*v2, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(val2);
        REQUIRE_CALL(*k3, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(key3);
        REQUIRE_CALL(*v3, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(val3);

        std::vector<ast::Map_Constructor::KV_Pair> pairs;
        pairs.emplace_back(std::move(k1), std::move(v1));
        pairs.emplace_back(std::move(k2), std::move(v2));
        pairs.emplace_back(std::move(k3), std::move(v3));
        ast::Map_Constructor node{std::move(pairs)};

        auto res = node.evaluate(syms);
        auto map = res->get<Map>().value();
        CHECK(map.size() == 3);
        CHECK(map.at(key1) == val1);
        CHECK(map.at(key2) == val2);
        CHECK(map.at(key3) == val3);
    }

    SECTION("Duplicate keys overwrite")
    {
        auto k1 = make();
        auto v1 = make();
        auto k2 = make();
        auto v2 = make();

        auto key1 = Value::create(1_f);
        auto key2 = Value::create(1_f);

        auto val1 = Value::create("first"s);
        auto val2 = Value::create("second"s);

        trompeloeil::sequence seq;

        REQUIRE_CALL(*k1, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(key1);
        REQUIRE_CALL(*v1, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(val1);
        REQUIRE_CALL(*k2, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(key2);
        REQUIRE_CALL(*v2, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(val2);

        std::vector<ast::Map_Constructor::KV_Pair> pairs;
        pairs.emplace_back(std::move(k1), std::move(v1));
        pairs.emplace_back(std::move(k2), std::move(v2));
        ast::Map_Constructor node{std::move(pairs)};

        auto res = node.evaluate(syms);
        auto map = res->get<Map>().value();
        CHECK(map.size() == 1);
        CHECK(map.at(Value::create(1_f)) == val2);
    }

    SECTION("Mixed numeric keys are distinct")
    {
        auto k1 = make();
        auto v1 = make();
        auto k2 = make();
        auto v2 = make();

        auto key_int = Value::create(1_f);
        auto key_float = Value::create(1.0);

        auto val_int = Value::create("int"s);
        auto val_float = Value::create("float"s);

        trompeloeil::sequence seq;

        REQUIRE_CALL(*k1, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(key_int);
        REQUIRE_CALL(*v1, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(val_int);
        REQUIRE_CALL(*k2, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(key_float);
        REQUIRE_CALL(*v2, evaluate(_))
            .LR_WITH(&_1 == &syms)
            .IN_SEQUENCE(seq)
            .RETURN(val_float);

        std::vector<ast::Map_Constructor::KV_Pair> pairs;
        pairs.emplace_back(std::move(k1), std::move(v1));
        pairs.emplace_back(std::move(k2), std::move(v2));
        ast::Map_Constructor node{std::move(pairs)};

        auto res = node.evaluate(syms);
        auto map = res->get<Map>().value();
        CHECK(map.size() == 2);
        CHECK(map.at(Value::create(1_f)) == val_int);
        CHECK(map.at(Value::create(1.0)) == val_float);
    }
}
