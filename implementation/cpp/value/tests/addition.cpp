#include <catch2/catch_test_macros.hpp>
#include <frost/value.hpp>

using namespace std::literals;
using namespace frst::literals;
using frst::Value, frst::Value_Ptr;

TEST_CASE("Numeric Add")
{
    auto int1 = Value::create(42_f);
    auto int2 = Value::create(81_f);
    auto flt1 = Value::create(3.14);
    auto flt2 = Value::create(2.17);

    SECTION("INT + INT")
    {
        auto res = Value::add(int1, int2);
        REQUIRE(res->is<frst::Int>());
        CHECK(res->get<frst::Int>().value() == 123_f);
    }

    SECTION("FLOAT + FLOAT")
    {
        auto res = Value::add(flt1, flt2);
        REQUIRE(res->is<frst::Float>());
        CHECK(res->get<frst::Float>().value() == 3.14 + 2.17);
    }

    SECTION("INT + FLOAT")
    {
        auto res = Value::add(int1, flt1);
        REQUIRE(res->is<frst::Float>());
        CHECK(res->get<frst::Float>().value() == 42_f + 3.14);
    }

    SECTION("FLOAT + INT")
    {
        auto res = Value::add(flt2, int2);
        REQUIRE(res->is<frst::Float>());
        CHECK(res->get<frst::Float>().value() == 2.17 + 81_f);
    }
}
