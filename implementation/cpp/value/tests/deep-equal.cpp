#include <catch2/catch_test_macros.hpp>

#include <array>
#include <memory_resource>

#include <frost/testing/stringmaker-specializations.hpp>
#include <frost/value.hpp>

using namespace frst;
using namespace std::literals;
using namespace frst::literals;

namespace
{
struct Dummy final : Callable
{
    Value_Ptr call(std::span<const Value_Ptr>) const override
    {
        return Value::null();
    }

    std::string debug_dump() const override
    {
        return "<dummy>";
    }
};

bool deep_eq(const Value_Ptr& lhs, const Value_Ptr& rhs)
{
    return Value::deep_equal(lhs, rhs)->get<Bool>().value();
}
} // namespace

TEST_CASE("Deep Equal")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).

    SECTION("Primitive values")
    {
        auto null1 = Value::null();
        auto null2 = Value::null();
        CHECK(deep_eq(null1, null2));

        auto i1 = Value::create(42_f);
        auto i2 = Value::create(42_f);
        auto i3 = Value::create(7_f);
        CHECK(deep_eq(i1, i2));
        CHECK_FALSE(deep_eq(i1, i3));

        auto f1 = Value::create(3.5);
        auto f2 = Value::create(3.5);
        auto f3 = Value::create(2.0);
        CHECK(deep_eq(f1, f2));
        CHECK_FALSE(deep_eq(f1, f3));

        auto b1 = Value::create(true);
        auto b2 = Value::create(true);
        auto b3 = Value::create(false);
        CHECK(deep_eq(b1, b2));
        CHECK_FALSE(deep_eq(b1, b3));

        auto s1 = Value::create("hello"s);
        auto s2 = Value::create("hello"s);
        auto s3 = Value::create("world"s);
        CHECK(deep_eq(s1, s2));
        CHECK_FALSE(deep_eq(s1, s3));
    }

    SECTION("Different types are always unequal")
    {
        auto i1 = Value::create(1_f);
        auto f1 = Value::create(1.0);
        auto b1 = Value::create(true);
        auto s1 = Value::create("1"s);
        auto n1 = Value::null();
        auto fn1 = Value::create(Function{std::make_shared<Dummy>()});

        CHECK_FALSE(deep_eq(i1, f1));
        CHECK_FALSE(deep_eq(i1, b1));
        CHECK_FALSE(deep_eq(i1, s1));
        CHECK_FALSE(deep_eq(i1, n1));
        CHECK_FALSE(deep_eq(i1, fn1));
        CHECK_FALSE(deep_eq(f1, s1));
        CHECK_FALSE(deep_eq(b1, n1));
        CHECK_FALSE(deep_eq(fn1, n1));
    }

    SECTION("Functions compare by identity only")
    {
        auto fn_ptr = std::make_shared<Dummy>();
        auto fn1 = Value::create(Function{fn_ptr});
        auto fn2 = Value::create(Function{fn_ptr});
        auto fn3 = Value::create(Function{std::make_shared<Dummy>()});

        CHECK(deep_eq(fn1, fn2));
        CHECK_FALSE(deep_eq(fn1, fn3));
    }

    SECTION("Arrays compare elementwise in order")
    {
        auto empty1 = Value::create(Array{});
        auto empty2 = Value::create(Array{});
        CHECK(deep_eq(empty1, empty2));

        auto v1a = Value::create(1_f);
        auto v2a = Value::create(2_f);
        auto v1b = Value::create(1_f);
        auto v2b = Value::create(2_f);

        auto arr1 = Value::create(Array{v1a, v2a});
        auto arr2 = Value::create(Array{v1b, v2b});
        auto arr3 = Value::create(Array{v2b, v1b});
        auto arr4 = Value::create(Array{v1a});

        CHECK(deep_eq(arr1, arr2));
        CHECK_FALSE(deep_eq(arr1, arr3));
        CHECK_FALSE(deep_eq(arr1, arr4));

        auto inner1 = Value::create(Array{Value::create(1_f)});
        auto inner2 = Value::create(Array{Value::create(2_f)});
        auto inner1b = Value::create(Array{Value::create(1_f)});

        auto nested1 = Value::create(Array{inner1, inner2});
        auto nested2 = Value::create(Array{inner1b, inner2});
        auto nested3 = Value::create(Array{inner1b, Value::create(Array{})});

        CHECK(deep_eq(nested1, nested2));
        CHECK_FALSE(deep_eq(nested1, nested3));
    }

    SECTION("Maps compare keys and values deeply")
    {
        auto empty1 = Value::create(Map{});
        auto empty2 = Value::create(Map{});
        CHECK(deep_eq(empty1, empty2));

        auto k1a = Value::create(1_f);
        auto v1a = Value::create(2_f);
        auto k1b = Value::create(1_f);
        auto v1b = Value::create(2_f);

        auto map1 = Value::create(Map{{k1a, v1a}});
        auto map2 = Value::create(Map{{k1b, v1b}});
        CHECK(deep_eq(map1, map2));

        auto map3 = Value::create(Map{{k1b, Value::create(3_f)}});
        CHECK_FALSE(deep_eq(map1, map3));

        auto map4 = Value::create(Map{{Value::create(2_f), v1b}});
        CHECK_FALSE(deep_eq(map1, map4));

        auto array_key1 = Value::create(Array{Value::create(1_f)});
        auto array_key2 = Value::create(Array{Value::create(1_f)});
        auto map5 = Value::create(Map{{array_key1, Value::create(9_f)}});
        auto map6 = Value::create(Map{{array_key2, Value::create(9_f)}});
        CHECK(deep_eq(map5, map6));

        auto nested_val1 = Value::create(Array{Value::create(1_f)});
        auto nested_val2 = Value::create(Array{Value::create(1_f)});
        auto map7 = Value::create(Map{{Value::create("k"s), nested_val1}});
        auto map8 = Value::create(Map{{Value::create("k"s), nested_val2}});
        CHECK(deep_eq(map7, map8));
    }

    SECTION("Recursive structures")
    {
        SECTION("Arrays containing maps and arrays compare deeply")
        {
            auto inner_arr1 =
                Value::create(Array{Value::create(1_f), Value::create(2_f)});
            auto inner_arr2 =
                Value::create(Array{Value::create(1_f), Value::create(2_f)});

            auto inner_map1 =
                Value::create(Map{{Value::create("k"s), inner_arr1}});
            auto inner_map2 =
                Value::create(Map{{Value::create("k"s), inner_arr2}});

            auto nested1 =
                Value::create(Array{Value::create(0_f), inner_map1});
            auto nested2 =
                Value::create(Array{Value::create(0_f), inner_map2});
            auto nested3 =
                Value::create(Array{Value::create(0_f),
                                    Value::create(Map{
                                        {Value::create("k"s),
                                         Value::create(Array{
                                             Value::create(1_f),
                                             Value::create(3_f),
                                         })},
                                    })});

            CHECK(deep_eq(nested1, nested2));
            CHECK_FALSE(deep_eq(nested1, nested3));
        }

        SECTION("Maps containing arrays and maps compare deeply")
        {
            auto map1 = Value::create(Map{
                {Value::create("a"s),
                 Value::create(Array{
                     Value::create(1_f),
                     Value::create(Array{Value::create(2_f)}),
                 })},
                {Value::create("b"s),
                 Value::create(Map{
                     {Value::create(1_f),
                      Value::create(Array{Value::create(3_f)})},
                 })},
            });

            auto map2 = Value::create(Map{
                {Value::create("a"s),
                 Value::create(Array{
                     Value::create(1_f),
                     Value::create(Array{Value::create(2_f)}),
                 })},
                {Value::create("b"s),
                 Value::create(Map{
                     {Value::create(1_f),
                      Value::create(Array{Value::create(3_f)})},
                 })},
            });

            auto map3 = Value::create(Map{
                {Value::create("a"s),
                 Value::create(Array{
                     Value::create(1_f),
                     Value::create(Array{Value::create(9_f)}),
                 })},
                {Value::create("b"s),
                 Value::create(Map{
                     {Value::create(1_f),
                      Value::create(Array{Value::create(3_f)})},
                 })},
            });

            CHECK(deep_eq(map1, map2));
            CHECK_FALSE(deep_eq(map1, map3));
        }

        SECTION("Structured keys are compared deeply")
        {
            auto key1 = Value::create(Array{
                Value::create(1_f),
                Value::create(Array{Value::create(2_f)}),
            });
            auto key2 = Value::create(Array{
                Value::create(1_f),
                Value::create(Array{Value::create(2_f)}),
            });

            auto val1 = Value::create(Map{
                {Value::create("k"s), Value::create(3_f)},
            });
            auto val2 = Value::create(Map{
                {Value::create("k"s), Value::create(3_f)},
            });

            auto map1 = Value::create(Map{{key1, val1}});
            auto map2 = Value::create(Map{{key2, val2}});
            auto map3 = Value::create(Map{
                {Value::create(Array{Value::create(1_f),
                                     Value::create(Array{Value::create(9_f)})}),
                 val2},
            });

            CHECK(deep_eq(map1, map2));
            CHECK_FALSE(deep_eq(map1, map3));
        }

        SECTION("Structured key ordering should not affect equality")
        {
            std::array<std::byte, 1024> buf1{};
            std::pmr::monotonic_buffer_resource res1{buf1.data(),
                                                     buf1.size()};
            std::pmr::polymorphic_allocator<Value> alloc1{&res1};

            std::array<std::byte, 1024> buf2{};
            std::pmr::monotonic_buffer_resource res2{buf2.data(),
                                                     buf2.size()};
            std::pmr::polymorphic_allocator<Value> alloc2{&res2};

            auto key_a1 =
                std::allocate_shared<Value>(alloc1,
                                            Array{Value::create(1_f)});
            auto key_b1 =
                std::allocate_shared<Value>(alloc1,
                                            Array{Value::create(2_f)});

            auto key_b2 =
                std::allocate_shared<Value>(alloc2,
                                            Array{Value::create(2_f)});
            auto key_a2 =
                std::allocate_shared<Value>(alloc2,
                                            Array{Value::create(1_f)});

            REQUIRE(key_a1.get() < key_b1.get());
            REQUIRE(key_b2.get() < key_a2.get());

            auto va1 = Value::create(10_f);
            auto va2 = Value::create(10_f);
            auto vb1 = Value::create(20_f);
            auto vb2 = Value::create(20_f);

            auto map1 = Value::create(Map{{key_a1, va1}, {key_b1, vb1}});
            auto map2 = Value::create(Map{{key_a2, va2}, {key_b2, vb2}});

            CHECK(deep_eq(map1, map2));
        }
    }
}
