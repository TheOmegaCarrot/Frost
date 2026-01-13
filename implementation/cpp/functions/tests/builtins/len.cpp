#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <frost/builtin.hpp>

using namespace frst;

using namespace std::literals;
using Catch::Matchers::ContainsSubstring;

TEST_CASE("Builtin len")
{
    // AI-generated test additions by Codex (GPT-5).
    // Signed: Codex (GPT-5).

    Symbol_Table table;
    inject_builtins(table);
    auto len_val = table.lookup("len");
    REQUIRE(len_val->is<Function>());
    auto len = len_val->get<Function>().value();

    SECTION("Injected")
    {
        CHECK(len_val->is<Function>());
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(len->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(len->call({Value::null(), Value::null()}),
                          ContainsSubstring("too many arguments"));
    }

    SECTION("Array length")
    {
        auto empty = Value::create(frst::Array{});
        auto three = Value::create(frst::Array{
            Value::create(1_f),
            Value::create("two"s),
            Value::create(true),
        });

        auto res_empty = len->call({empty});
        REQUIRE(res_empty->is<frst::Int>());
        CHECK(res_empty->get<frst::Int>().value() == 0);

        auto res_three = len->call({three});
        REQUIRE(res_three->is<frst::Int>());
        CHECK(res_three->get<frst::Int>().value() == 3);
    }

    SECTION("Map length")
    {
        auto empty = Value::create(frst::Map{});
        auto two = Value::create(frst::Map{
            {Value::create("a"s), Value::create(1_f)},
            {Value::create("b"s), Value::create(2_f)},
        });

        auto res_empty = len->call({empty});
        REQUIRE(res_empty->is<frst::Int>());
        CHECK(res_empty->get<frst::Int>().value() == 0);

        auto res_two = len->call({two});
        REQUIRE(res_two->is<frst::Int>());
        CHECK(res_two->get<frst::Int>().value() == 2);
    }

    SECTION("String length (bytes)")
    {
        auto empty = Value::create(""s);
        auto ascii = Value::create("hello"s);
        auto bytes = Value::create(std::string{"a\0b", 3});

        auto res_empty = len->call({empty});
        REQUIRE(res_empty->is<frst::Int>());
        CHECK(res_empty->get<frst::Int>().value() == 0);

        auto res_ascii = len->call({ascii});
        REQUIRE(res_ascii->is<frst::Int>());
        CHECK(res_ascii->get<frst::Int>().value() == 5);

        auto res_bytes = len->call({bytes});
        REQUIRE(res_bytes->is<frst::Int>());
        CHECK(res_bytes->get<frst::Int>().value() == 3);
    }

    SECTION("Type error")
    {
        auto bad = Value::create(42_f);
        try
        {
            len->call({bad});
            FAIL("Expected type error");
        }
        catch (const Frost_Error& err)
        {
            const std::string msg = err.what();
            CHECK_THAT(msg, ContainsSubstring("Function len"));
            CHECK_THAT(msg, ContainsSubstring("Map or Array or String"));
            CHECK_THAT(msg, ContainsSubstring("argument 1"));
            CHECK_THAT(msg, ContainsSubstring("got Int"));
        }
    }
}
