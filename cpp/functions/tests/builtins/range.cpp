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
                                       Value::create(3_f), Value::create(4_f)}),
                          ContainsSubstring("too many arguments"));
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

        SECTION("Three-arg: start, stop, step must all be Int")
        {
            auto i = Value::create(1_f);
            auto f = Value::create(3.14);
            CHECK_THROWS_WITH(range->call({f, i, i}),
                              ContainsSubstring("start")
                                  && ContainsSubstring("Int"));
            CHECK_THROWS_WITH(range->call({i, f, i}),
                              ContainsSubstring("stop")
                                  && ContainsSubstring("Int"));
            CHECK_THROWS_WITH(range->call({i, i, f}),
                              ContainsSubstring("step")
                                  && ContainsSubstring("Int"));
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

    SECTION("Three-arg semantics")
    {
        auto call3 = [&](Int a, Int b, Int c) {
            return range->call(
                {Value::create(a), Value::create(b), Value::create(c)});
        };
        auto ints = [](const Value_Ptr& v) {
            REQUIRE(v->is<Array>());
            std::vector<Int> out;
            for (const auto& e : v->get<Array>().value())
                out.push_back(e->get<Int>().value());
            return out;
        };
        auto empty_arr = [](const Value_Ptr& v) -> bool {
            REQUIRE(v->is<Array>());
            return v->get<Array>().value().empty();
        };

        SECTION("step == 0 produces an error")
        {
            CHECK_THROWS_MATCHES(
                call3(0, 10, 0), Frost_User_Error,
                MessageMatches(ContainsSubstring("step != 0")));
        }

        SECTION("step=1 matches two-arg form")
        {
            CHECK(ints(call3(2, 5, 1)) == std::vector<Int>{2, 3, 4});
            CHECK(ints(call3(-2, 2, 1)) == std::vector<Int>{-2, -1, 0, 1});
        }

        SECTION("positive step skips elements")
        {
            CHECK(ints(call3(0, 10, 2)) == std::vector<Int>{0, 2, 4, 6, 8});
            CHECK(ints(call3(0, 10, 3)) == std::vector<Int>{0, 3, 6, 9});
        }

        SECTION("positive step larger than span yields one element")
        {
            CHECK(ints(call3(0, 3, 100)) == std::vector<Int>{0});
            CHECK(ints(call3(7, 8, 100)) == std::vector<Int>{7});
        }

        SECTION("positive step with start >= stop returns empty")
        {
            CHECK(empty_arr(call3(5, 5, 1)));
            CHECK(empty_arr(call3(6, 5, 1)));
        }

        SECTION("negative step counts down")
        {
            CHECK(ints(call3(10, 0, -1))
                  == std::vector<Int>{10, 9, 8, 7, 6, 5, 4, 3, 2, 1});
            CHECK(ints(call3(10, 0, -2)) == std::vector<Int>{10, 8, 6, 4, 2});
            CHECK(ints(call3(5, -1, -2)) == std::vector<Int>{5, 3, 1});
        }

        SECTION("negative step on all-negative inputs")
        {
            CHECK(ints(call3(-1, -5, -1)) == std::vector<Int>{-1, -2, -3, -4});
            CHECK(ints(call3(-2, -8, -2)) == std::vector<Int>{-2, -4, -6});
        }

        SECTION("negative step yields single element")
        {
            CHECK(ints(call3(5, 4, -1)) == std::vector<Int>{5});
            CHECK(ints(call3(-3, -4, -1)) == std::vector<Int>{-3});
        }

        SECTION("negative step with start <= stop returns empty")
        {
            CHECK(empty_arr(call3(0, 5, -1)));
            CHECK(empty_arr(call3(5, 5, -1)));
        }
    }
}
