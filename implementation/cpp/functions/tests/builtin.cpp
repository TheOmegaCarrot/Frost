#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <frost/builtin.hpp>

using namespace frst;

using namespace std::literals;
using Catch::Matchers::ContainsSubstring;

TEST_CASE("Builtin Function")
{
    // AI-generated test additions by Codex (GPT-5).
    // Signed: Codex (GPT-5).

    SECTION("debug_dump format")
    {
        Builtin builtin{
            [](builtin_args_t) {
                return Value::null();
            },
            "debug",
            Builtin::Arity{0, 0},
        };

        CHECK(builtin.debug_dump() == "<builtin:debug>");
    }

    SECTION("arity: zero-argument builtin succeeds")
    {
        bool called = false;
        Builtin builtin{
            [&](builtin_args_t) {
                called = true;
                return Value::null();
            },
            "zero",
            Builtin::Arity{0, 0},
        };

        std::vector<Value_Ptr> args{};
        REQUIRE_NOTHROW(builtin.call(args));
        CHECK(called);
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
                same_address = (&in == expected);
                return Value::null();
            },
            "forward",
            Builtin::Arity{0, std::nullopt},
        };

        REQUIRE_NOTHROW(builtin.call(args));
        CHECK(same_address);
    }

    SECTION("arity: within bounds succeeds (min and max)")
    {
        int calls = 0;
        Builtin builtin{
            [&](builtin_args_t) {
                ++calls;
                return Value::null();
            },
            "bounded",
            Builtin::Arity{1, 3},
        };

        std::vector<Value_Ptr> one{
            Value::create(1_f),
        };
        std::vector<Value_Ptr> three{
            Value::create(1_f),
            Value::create(2_f),
            Value::create(3_f),
        };

        REQUIRE_NOTHROW(builtin.call(one));
        CHECK(calls == 1);
        REQUIRE_NOTHROW(builtin.call(three));
        CHECK(calls == 2);
    }

    SECTION("arity: too many arguments")
    {
        bool called = false;
        Builtin builtin{
            [&](builtin_args_t) {
                called = true;
                return Value::null();
            },
            "too_many",
            Builtin::Arity{1, 1},
        };

        std::vector<Value_Ptr> args{
            Value::create(1_f),
            Value::create(2_f),
        };

        try
        {
            builtin.call(args);
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
        CHECK_FALSE(called);
    }

    SECTION("arity: insufficient arguments")
    {
        bool called = false;
        Builtin builtin{
            [&](builtin_args_t) {
                called = true;
                return Value::null();
            },
            "too_few",
            Builtin::Arity{2, 4},
        };

        std::vector<Value_Ptr> args{
            Value::create(1_f),
        };

        try
        {
            builtin.call(args);
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
        CHECK_FALSE(called);
    }

    SECTION("arity: nullopt max has no upper bound")
    {
        bool called = false;
        Builtin builtin{
            [&](builtin_args_t) {
                called = true;
                return Value::null();
            },
            "variadic",
            Builtin::Arity{1, std::nullopt},
        };

        std::vector<Value_Ptr> args{
            Value::create(1_f),
            Value::create(2_f),
            Value::create(3_f),
            Value::create(4_f),
        };

        REQUIRE_NOTHROW(builtin.call(args));
        CHECK(called);
    }
}
