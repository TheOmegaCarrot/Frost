#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/builtins-common.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <frost/builtin.hpp>

using namespace frst;
using namespace frst::builtin_detail;

using namespace std::literals;
using Catch::Matchers::ContainsSubstring;

TEST_CASE("Builtin Function")
{
    SECTION("debug_dump format")
    {
        Builtin builtin{
            [](builtin_args_t) {
                return Value::null();
            },
            "debug",
        };

        CHECK(builtin.debug_dump() == "<builtin:debug>");
    }

    SECTION("call forwards args by reference")
    {
        std::vector<Value_Ptr> args{
            Value::create(1_f),
            Value::create("two"s),
        };
        const auto* expected = &args;
        bool same_address = false;

        Builtin builtin{
            [&](builtin_args_t in) {
                same_address = in.data()
                               == expected->data()
                               && in.size()
                               == expected->size();
                return Value::null();
            },
            "forward",
        };

        REQUIRE_NOTHROW(builtin.call(args));
        CHECK(same_address);
    }

    SECTION("call invokes the function")
    {
        bool called = false;
        Builtin builtin{
            [&](builtin_args_t) {
                called = true;
                return Value::null();
            },
            "test",
        };

        std::vector<Value_Ptr> args{};
        REQUIRE_NOTHROW(builtin.call(args));
        CHECK(called);
    }

    SECTION("call returns the function result")
    {
        Builtin builtin{
            [](builtin_args_t) {
                return Value::create(42_f);
            },
            "returns_42",
        };

        std::vector<Value_Ptr> args{};
        auto result = builtin.call(args);
        CHECK(result->get<Int>() == 42);
    }
}

TEST_CASE("require_arity")
{
    SECTION("zero-argument succeeds")
    {
        std::vector<Value_Ptr> args{};
        REQUIRE_NOTHROW(require_arity("zero", args, 0, 0));
    }

    SECTION("within bounds succeeds (min and max)")
    {
        std::vector<Value_Ptr> one{
            Value::create(1_f),
        };
        std::vector<Value_Ptr> three{
            Value::create(1_f),
            Value::create(2_f),
            Value::create(3_f),
        };

        REQUIRE_NOTHROW(require_arity("bounded", one, 1, 3));
        REQUIRE_NOTHROW(require_arity("bounded", three, 1, 3));
    }

    SECTION("too many arguments")
    {
        std::vector<Value_Ptr> args{
            Value::create(1_f),
            Value::create(2_f),
        };

        try
        {
            require_arity("too_many", args, 1, 1);
            FAIL("Expected too-many-arguments error");
        }
        catch (const Frost_User_Error& err)
        {
            const std::string msg = err.what();
            CHECK_THAT(msg, ContainsSubstring("Function too_many"));
            CHECK_THAT(msg, ContainsSubstring("too many arguments"));
            CHECK_THAT(msg, ContainsSubstring("Called with 2"));
            CHECK_THAT(msg, ContainsSubstring("no more than 1"));
        }
    }

    SECTION("insufficient arguments")
    {
        std::vector<Value_Ptr> args{
            Value::create(1_f),
        };

        try
        {
            require_arity("too_few", args, 2, 4);
            FAIL("Expected insufficient-arguments error");
        }
        catch (const Frost_User_Error& err)
        {
            const std::string msg = err.what();
            CHECK_THAT(msg, ContainsSubstring("Function too_few"));
            CHECK_THAT(msg, ContainsSubstring("insufficient arguments"));
            CHECK_THAT(msg, ContainsSubstring("Called with 1"));
            CHECK_THAT(msg, ContainsSubstring("at least 2"));
        }
    }

    SECTION("nullopt max has no upper bound")
    {
        std::vector<Value_Ptr> args{
            Value::create(1_f),
            Value::create(2_f),
            Value::create(3_f),
            Value::create(4_f),
        };

        REQUIRE_NOTHROW(require_arity("variadic", args, 1));
    }
}
