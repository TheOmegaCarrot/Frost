#include <catch2/catch_all.hpp>

#include <frost/value.hpp>

#include <frost/extensions-common.hpp>

using namespace frst;

namespace frst
{
DECLARE_EXTENSION(unsafe);
}

namespace
{

Function lookup(const std::string& name)
{
    static Map ext = make_extension_unsafe();
    auto key = Value::create(String{name});
    return ext.at(key)->raw_get<Function>();
}

} // namespace

TEST_CASE("unsafe.same")
{
    auto same = lookup("same");

    SECTION("same value is same")
    {
        auto a = Value::create(Int{5});
        auto result = same->call({a, a});
        CHECK(result->get<Bool>().value() == true);
    }

    SECTION("equal but distinct values are not same")
    {
        auto a = Value::create(Int{5});
        auto b = Value::create(Int{5});
        auto result = same->call({a, b});
        CHECK(result->get<Bool>().value() == false);
    }

    SECTION("alias is same")
    {
        auto a = Value::create(String{"hello"});
        auto c = a;
        auto result = same->call({a, c});
        CHECK(result->get<Bool>().value() == true);
    }

    SECTION("different types are not same")
    {
        auto a = Value::create(Int{1});
        auto b = Value::create(Float{1.0});
        auto result = same->call({a, b});
        CHECK(result->get<Bool>().value() == false);
    }

    SECTION("null singletons are same")
    {
        auto a = Value::null();
        auto b = Value::null();
        auto result = same->call({a, b});
        CHECK(result->get<Bool>().value() == true);
    }

    SECTION("bool singletons are same")
    {
        auto a = Value::create(Bool{true});
        auto b = Value::create(Bool{true});
        auto result = same->call({a, b});
        CHECK(result->get<Bool>().value() == true);
    }

    SECTION("true and false are not same")
    {
        auto a = Value::create(Bool{true});
        auto b = Value::create(Bool{false});
        auto result = same->call({a, b});
        CHECK(result->get<Bool>().value() == false);
    }
}

TEST_CASE("unsafe.identity")
{
    auto identity = lookup("identity");

    SECTION("returns an Int")
    {
        auto a = Value::create(Int{42});
        auto result = identity->call({a});
        CHECK(result->get<Int>().has_value());
    }

    SECTION("same value gives same address")
    {
        auto a = Value::create(Int{42});
        auto r1 = identity->call({a});
        auto r2 = identity->call({a});
        CHECK(r1->get<Int>().value() == r2->get<Int>().value());
    }

    SECTION("alias gives same address")
    {
        auto a = Value::create(String{"hello"});
        auto c = a;
        auto r1 = identity->call({a});
        auto r2 = identity->call({c});
        CHECK(r1->get<Int>().value() == r2->get<Int>().value());
    }

    SECTION("distinct values give different addresses")
    {
        auto a = Value::create(Int{5});
        auto b = Value::create(Int{5});
        auto r1 = identity->call({a});
        auto r2 = identity->call({b});
        CHECK(r1->get<Int>().value() != r2->get<Int>().value());
    }

    SECTION("consistent with same")
    {
        auto same = lookup("same");
        auto a = Value::create(Int{5});
        auto b = Value::create(Int{5});

        bool same_says = same->call({a, b})->get<Bool>().value();
        bool ids_equal = identity->call({a})->get<Int>().value()
                         == identity->call({b})->get<Int>().value();
        CHECK(same_says == ids_equal);
    }
}
