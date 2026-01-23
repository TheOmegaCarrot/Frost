#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_exception.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>
#include <catch2/trompeloeil.hpp>

#include <vector>

#include <frost/testing/stringmaker-specializations.hpp>
#include <frost/value.hpp>

using namespace frst;
using namespace std::literals;
using namespace frst::literals;
using Catch::Matchers::ContainsSubstring;
using Catch::Matchers::MessageMatches;
using trompeloeil::_;

namespace
{
class Mock_Callable final : public Callable
{
  public:
    MAKE_CONST_MOCK(call, auto(std::span<const Value_Ptr>)->Value_Ptr,
                    override);
    MAKE_CONST_MOCK(debug_dump, auto()->std::string, override);

    using Ptr = std::shared_ptr<Mock_Callable>;

    static Ptr make()
    {
        return std::make_shared<Mock_Callable>();
    }
};

void require_array_eq(const Value_Ptr& value,
                      const std::vector<Value_Ptr>& expected)
{
    REQUIRE(value->is<Array>());
    const auto& arr = value->raw_get<Array>();
    REQUIRE(arr.size() == expected.size());
    for (std::size_t i = 0; i < expected.size(); ++i)
        CHECK(arr.at(i) == expected.at(i));
}
} // namespace

TEST_CASE("Value iterative ops")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).

    SECTION("do_map array")
    {
        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto arr = Value::create(Array{a, b});

        auto callable = Mock_Callable::make();
        Function op = callable;
        trompeloeil::sequence seq;

        REQUIRE_CALL(*callable, call(_))
            .IN_SEQUENCE(seq)
            .LR_SIDE_EFFECT({
                CHECK(_1.size() == 1);
                CHECK(_1.at(0) == a);
            })
            .RETURN(Value::create(2_f));

        REQUIRE_CALL(*callable, call(_))
            .IN_SEQUENCE(seq)
            .LR_SIDE_EFFECT({
                CHECK(_1.size() == 1);
                CHECK(_1.at(0) == b);
            })
            .RETURN(Value::create(3_f));

        auto res = Value::do_map(arr, op, "map");
        REQUIRE(res->is<Array>());
        const auto& out = res->raw_get<Array>();
        REQUIRE(out.size() == 2);
        CHECK(out.at(0)->raw_get<Int>() == 2_f);
        CHECK(out.at(1)->raw_get<Int>() == 3_f);
    }

    SECTION("do_map empty array returns same pointer")
    {
        auto empty = Value::create(Array{});
        auto callable = Mock_Callable::make();
        Function op = callable;

        FORBID_CALL(*callable, call(_));

        auto res = Value::do_map(empty, op, "map");
        CHECK(res == empty);
    }

    SECTION("do_map map merges intermediates")
    {
        auto key_a = Value::create("a"s);
        auto key_b = Value::create("b"s);
        auto val_a = Value::create(1_f);
        auto val_b = Value::create(2_f);
        auto map = Value::create(Map{{key_a, val_a}, {key_b, val_b}});

        auto callable = Mock_Callable::make();
        Function op = callable;
        trompeloeil::sequence seq;

        REQUIRE_CALL(*callable, call(_))
            .IN_SEQUENCE(seq)
            .LR_SIDE_EFFECT({
                CHECK(_1.size() == 2);
                CHECK(_1.at(0) == key_a);
                CHECK(_1.at(1) == val_a);
            })
            .RETURN(Value::create(Map{{key_a, Value::create(2_f)}}));

        REQUIRE_CALL(*callable, call(_))
            .IN_SEQUENCE(seq)
            .LR_SIDE_EFFECT({
                CHECK(_1.size() == 2);
                CHECK(_1.at(0) == key_b);
                CHECK(_1.at(1) == val_b);
            })
            .RETURN(Value::create(Map{{key_b, Value::create(4_f)}}));

        auto res = Value::do_map(map, op, "map");
        REQUIRE(res->is<Map>());
        const auto& out = res->raw_get<Map>();
        REQUIRE(out.size() == 2);
        auto it_a = out.find(key_a);
        auto it_b = out.find(key_b);
        REQUIRE(it_a != out.end());
        REQUIRE(it_b != out.end());
        CHECK(it_a->first == key_a);
        CHECK(it_b->first == key_b);
        CHECK(it_a->second->raw_get<Int>() == 2_f);
        CHECK(it_b->second->raw_get<Int>() == 4_f);
    }

    SECTION("do_map map rejects non-map intermediates")
    {
        auto key = Value::create("k"s);
        auto map = Value::create(Map{{key, Value::create(1_f)}});

        auto callable = Mock_Callable::make();
        Function op = callable;
        ALLOW_CALL(*callable, call(_)).RETURN(Value::create(1_f));

        CHECK_THROWS_MATCHES(
            Value::do_map(map, op, "map"), Frost_Recoverable_Error,
            MessageMatches(
                ContainsSubstring("map with map input requires map intermediates")
                && ContainsSubstring("got Int")));
    }

    SECTION("do_map map rejects key collisions")
    {
        auto map = Value::create(Map{
            {Value::create("a"s), Value::create(1_f)},
            {Value::create("b"s), Value::create(2_f)},
        });

        auto callable = Mock_Callable::make();
        Function op = callable;
        ALLOW_CALL(*callable, call(_))
            .RETURN(Value::create(
                Map{{Value::create("k"s), Value::create(1_f)}}));

        CHECK_THROWS_WITH(Value::do_map(map, op, "map"),
                          ContainsSubstring("key collision"));
    }

    SECTION("do_filter array")
    {
        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto c = Value::create(3_f);
        auto arr = Value::create(Array{a, b, c});

        auto callable = Mock_Callable::make();
        Function pred = callable;
        trompeloeil::sequence seq;

        REQUIRE_CALL(*callable, call(_))
            .IN_SEQUENCE(seq)
            .LR_SIDE_EFFECT({ CHECK(_1.at(0) == a); })
            .RETURN(Value::create(false));
        REQUIRE_CALL(*callable, call(_))
            .IN_SEQUENCE(seq)
            .LR_SIDE_EFFECT({ CHECK(_1.at(0) == b); })
            .RETURN(Value::create(true));
        REQUIRE_CALL(*callable, call(_))
            .IN_SEQUENCE(seq)
            .LR_SIDE_EFFECT({ CHECK(_1.at(0) == c); })
            .RETURN(Value::create(true));

        auto res = Value::do_filter(arr, pred);
        require_array_eq(res, {b, c});
        CHECK(res->raw_get<Array>().at(0) == b);
        CHECK(res->raw_get<Array>().at(1) == c);
    }

    SECTION("do_filter map preserves keys and values")
    {
        auto key_a = Value::create("a"s);
        auto key_b = Value::create("b"s);
        auto val_a = Value::create(1_f);
        auto val_b = Value::create(2_f);
        auto map = Value::create(Map{{key_a, val_a}, {key_b, val_b}});

        auto callable = Mock_Callable::make();
        Function pred = callable;
        trompeloeil::sequence seq;

        REQUIRE_CALL(*callable, call(_))
            .IN_SEQUENCE(seq)
            .LR_SIDE_EFFECT({
                CHECK(_1.at(0) == key_a);
                CHECK(_1.at(1) == val_a);
            })
            .RETURN(Value::create(false));
        REQUIRE_CALL(*callable, call(_))
            .IN_SEQUENCE(seq)
            .LR_SIDE_EFFECT({
                CHECK(_1.at(0) == key_b);
                CHECK(_1.at(1) == val_b);
            })
            .RETURN(Value::create(true));

        auto res = Value::do_filter(map, pred);
        REQUIRE(res->is<Map>());
        const auto& out = res->raw_get<Map>();
        REQUIRE(out.size() == 1);
        auto it = out.find(key_b);
        REQUIRE(it != out.end());
        CHECK(it->first == key_b);
        CHECK(it->second == val_b);
    }

    SECTION("do_reduce array")
    {
        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto c = Value::create(3_f);
        auto arr = Value::create(Array{a, b, c});

        auto callable = Mock_Callable::make();
        Function op = callable;
        trompeloeil::sequence seq;

        auto sum1 = Value::create(3_f);
        auto sum2 = Value::create(6_f);

        REQUIRE_CALL(*callable, call(_))
            .IN_SEQUENCE(seq)
            .LR_SIDE_EFFECT({
                CHECK(_1.at(0) == a);
                CHECK(_1.at(1) == b);
            })
            .RETURN(sum1);
        REQUIRE_CALL(*callable, call(_))
            .IN_SEQUENCE(seq)
            .LR_SIDE_EFFECT({
                CHECK(_1.at(0) == sum1);
                CHECK(_1.at(1) == c);
            })
            .RETURN(sum2);

        auto res = Value::do_reduce(arr, op, std::nullopt);
        CHECK(res == sum2);
    }

    SECTION("do_reduce array with init")
    {
        auto a = Value::create(1_f);
        auto b = Value::create(2_f);
        auto c = Value::create(3_f);
        auto arr = Value::create(Array{a, b, c});

        auto callable = Mock_Callable::make();
        Function op = callable;
        trompeloeil::sequence seq;

        auto init = Value::create(10_f);
        auto sum1 = Value::create(11_f);
        auto sum2 = Value::create(13_f);
        auto sum3 = Value::create(16_f);

        REQUIRE_CALL(*callable, call(_))
            .IN_SEQUENCE(seq)
            .LR_SIDE_EFFECT({
                CHECK(_1.at(0) == init);
                CHECK(_1.at(1) == a);
            })
            .RETURN(sum1);
        REQUIRE_CALL(*callable, call(_))
            .IN_SEQUENCE(seq)
            .LR_SIDE_EFFECT({
                CHECK(_1.at(0) == sum1);
                CHECK(_1.at(1) == b);
            })
            .RETURN(sum2);
        REQUIRE_CALL(*callable, call(_))
            .IN_SEQUENCE(seq)
            .LR_SIDE_EFFECT({
                CHECK(_1.at(0) == sum2);
                CHECK(_1.at(1) == c);
            })
            .RETURN(sum3);

        auto res = Value::do_reduce(arr, op, init);
        CHECK(res == sum3);
    }

    SECTION("do_reduce array empty")
    {
        auto empty = Value::create(Array{});
        auto callable = Mock_Callable::make();
        Function op = callable;

        FORBID_CALL(*callable, call(_));

        auto res_empty = Value::do_reduce(empty, op, std::nullopt);
        CHECK(res_empty == Value::null());

        auto init = Value::create(10_f);
        auto res_empty_init = Value::do_reduce(empty, op, init);
        CHECK(res_empty_init == init);
    }

    SECTION("do_reduce map")
    {
        auto key_a = Value::create("a"s);
        auto key_b = Value::create("b"s);
        auto val_a = Value::create(1_f);
        auto val_b = Value::create(2_f);
        auto map = Value::create(Map{{key_a, val_a}, {key_b, val_b}});

        auto callable = Mock_Callable::make();
        Function op = callable;
        trompeloeil::sequence seq;

        auto init = Value::create(10_f);
        auto sum1 = Value::create(11_f);
        auto sum2 = Value::create(13_f);

        REQUIRE_CALL(*callable, call(_))
            .IN_SEQUENCE(seq)
            .LR_SIDE_EFFECT({
                CHECK(_1.at(0) == init);
                CHECK(_1.at(1) == key_a);
                CHECK(_1.at(2) == val_a);
            })
            .RETURN(sum1);
        REQUIRE_CALL(*callable, call(_))
            .IN_SEQUENCE(seq)
            .LR_SIDE_EFFECT({
                CHECK(_1.at(0) == sum1);
                CHECK(_1.at(1) == key_b);
                CHECK(_1.at(2) == val_b);
            })
            .RETURN(sum2);

        auto res = Value::do_reduce(map, op, init);
        CHECK(res == sum2);
    }

    SECTION("do_reduce map empty")
    {
        auto empty = Value::create(Map{});
        auto callable = Mock_Callable::make();
        Function op = callable;
        auto init = Value::create(10_f);

        FORBID_CALL(*callable, call(_));

        auto res_empty = Value::do_reduce(empty, op, init);
        CHECK(res_empty == init);
    }

    SECTION("do_reduce map requires init")
    {
        auto map = Value::create(Map{{Value::create("a"s), Value::create(1_f)}});
        auto callable = Mock_Callable::make();
        Function op = callable;

        FORBID_CALL(*callable, call(_));

        CHECK_THROWS_WITH(Value::do_reduce(map, op, std::nullopt),
                          ContainsSubstring("Map reduction requires init"));
    }

    SECTION("do_* reject non-structured input")
    {
        auto non = Value::create(1_f);
        auto callable = Mock_Callable::make();
        Function op = callable;

        FORBID_CALL(*callable, call(_));

        CHECK_THROWS_AS(Value::do_map(non, op, "map"), Frost_Internal_Error);
        CHECK_THROWS_AS(Value::do_filter(non, op), Frost_Internal_Error);
        CHECK_THROWS_AS(Value::do_reduce(non, op, std::nullopt),
                        Frost_Internal_Error);
    }
}
