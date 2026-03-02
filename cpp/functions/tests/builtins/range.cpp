#include <catch2/catch_all.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <frost/builtin.hpp>

using namespace frst;

using namespace std::literals;

using namespace Catch::Matchers;

TEST_CASE("Builtin range")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    Symbol_Table table;
    inject_builtins(table);
    auto range_val = table.lookup("range");
    REQUIRE(range_val->is<Function>());
    auto range = range_val->get<Function>().value();

    SECTION("Injected")
    {
        CHECK(range_val->is<Function>());
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(range->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(range->call({}), ContainsSubstring("Called with 0"));
        CHECK_THROWS_WITH(range->call({}),
                          ContainsSubstring("requires at least 1"));

        CHECK_THROWS_WITH(range->call({Value::create(1_f), Value::create(2_f),
                                       Value::create(3_f)}),
                          ContainsSubstring("too many arguments"));
        CHECK_THROWS_WITH(range->call({Value::create(1_f), Value::create(2_f),
                                       Value::create(3_f)}),
                          ContainsSubstring("Called with 3"));
        CHECK_THROWS_WITH(range->call({Value::create(1_f), Value::create(2_f),
                                       Value::create(3_f)}),
                          ContainsSubstring("no more than 2"));
    }

    SECTION("Type checks")
    {
        SECTION("One-arg: upper bound must be Int")
        {
            auto bad_vals = std::vector<Value_Ptr>{
                Value::null(),          Value::create(3.14),
                Value::create(true),    Value::create("nope"s),
                Value::create(Array{}), Value::create(Map{}),
            };

            for (const auto& val : bad_vals)
            {
                CHECK_THROWS_WITH(range->call({val}),
                                  ContainsSubstring("Function range requires"));
                CHECK_THROWS_WITH(range->call({val}),
                                  ContainsSubstring("upper bound"));
                CHECK_THROWS_WITH(range->call({val}),
                                  EndsWith(std::string{val->type_name()}));
            }
        }

        SECTION("Two-arg: lower and upper bounds must be Int")
        {
            auto bad_lower = Value::create(3.14);
            auto good_upper = Value::create(10_f);
            CHECK_THROWS_WITH(range->call({bad_lower, good_upper}),
                              ContainsSubstring("Function range requires"));
            CHECK_THROWS_WITH(range->call({bad_lower, good_upper}),
                              ContainsSubstring("lower bound"));
            CHECK_THROWS_WITH(range->call({bad_lower, good_upper}),
                              EndsWith(std::string{bad_lower->type_name()}));

            auto good_lower = Value::create(1_f);
            auto bad_upper = Value::create("nope"s);
            CHECK_THROWS_WITH(range->call({good_lower, bad_upper}),
                              ContainsSubstring("Function range requires"));
            CHECK_THROWS_WITH(range->call({good_lower, bad_upper}),
                              ContainsSubstring("upper bound"));
            CHECK_THROWS_WITH(range->call({good_lower, bad_upper}),
                              EndsWith(std::string{bad_upper->type_name()}));
        }
    }

    SECTION("One-arg semantics")
    {
        SECTION("range(0) returns empty")
        {
            auto res = range->call({Value::create(0_f)});
            REQUIRE(res->is<Array>());
            CHECK(res->get<Array>().value().empty());
        }

        SECTION("range(3) returns [0, 1, 2]")
        {
            auto res = range->call({Value::create(3_f)});
            REQUIRE(res->is<Array>());
            auto arr = res->get<Array>().value();
            REQUIRE(arr.size() == 3);
            CHECK(arr.at(0)->get<Int>().value() == 0);
            CHECK(arr.at(1)->get<Int>().value() == 1);
            CHECK(arr.at(2)->get<Int>().value() == 2);
        }

        SECTION("range(1) returns [0]")
        {
            auto res = range->call({Value::create(1_f)});
            REQUIRE(res->is<Array>());
            auto arr = res->get<Array>().value();
            REQUIRE(arr.size() == 1);
            CHECK(arr.at(0)->get<Int>().value() == 0);
        }

        SECTION("range(-1) returns empty")
        {
            auto res = range->call({Value::create(-1_f)});
            REQUIRE(res->is<Array>());
            CHECK(res->get<Array>().value().empty());
        }
    }

    SECTION("Two-arg semantics")
    {
        SECTION("range(2, 5) returns [2, 3, 4]")
        {
            auto res = range->call({Value::create(2_f), Value::create(5_f)});
            REQUIRE(res->is<Array>());
            auto arr = res->get<Array>().value();
            REQUIRE(arr.size() == 3);
            CHECK(arr.at(0)->get<Int>().value() == 2);
            CHECK(arr.at(1)->get<Int>().value() == 3);
            CHECK(arr.at(2)->get<Int>().value() == 4);
        }

        SECTION("range(5, 2) returns empty")
        {
            auto res = range->call({Value::create(5_f), Value::create(2_f)});
            REQUIRE(res->is<Array>());
            CHECK(res->get<Array>().value().empty());
        }

        SECTION("range(5, 5) returns empty")
        {
            auto res = range->call({Value::create(5_f), Value::create(5_f)});
            REQUIRE(res->is<Array>());
            CHECK(res->get<Array>().value().empty());
        }

        SECTION("range(-2, 2) returns [-2, -1, 0, 1]")
        {
            auto res = range->call({Value::create(-2_f), Value::create(2_f)});
            REQUIRE(res->is<Array>());
            auto arr = res->get<Array>().value();
            REQUIRE(arr.size() == 4);
            CHECK(arr.at(0)->get<Int>().value() == -2);
            CHECK(arr.at(1)->get<Int>().value() == -1);
            CHECK(arr.at(2)->get<Int>().value() == 0);
            CHECK(arr.at(3)->get<Int>().value() == 1);
        }

        SECTION("range(-3, -1) returns [-3, -2]")
        {
            auto res = range->call({Value::create(-3_f), Value::create(-1_f)});
            REQUIRE(res->is<Array>());
            auto arr = res->get<Array>().value();
            REQUIRE(arr.size() == 2);
            CHECK(arr.at(0)->get<Int>().value() == -3);
            CHECK(arr.at(1)->get<Int>().value() == -2);
        }

        SECTION("range(-1, -3) returns empty")
        {
            auto res = range->call({Value::create(-1_f), Value::create(-3_f)});
            REQUIRE(res->is<Array>());
            CHECK(res->get<Array>().value().empty());
        }

        SECTION("range(0, 1) returns [0]")
        {
            auto res = range->call({Value::create(0_f), Value::create(1_f)});
            REQUIRE(res->is<Array>());
            auto arr = res->get<Array>().value();
            REQUIRE(arr.size() == 1);
            CHECK(arr.at(0)->get<Int>().value() == 0);
        }
    }
}
