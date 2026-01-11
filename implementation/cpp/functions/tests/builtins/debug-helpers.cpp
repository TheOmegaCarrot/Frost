#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <frost/builtin.hpp>

using namespace frst;

using namespace std::literals;
using Catch::Matchers::ContainsSubstring;

TEST_CASE("Builtin debug_dump")
{
    // AI-generated test additions by Codex (GPT-5).
    // Signed: Codex (GPT-5).

    Symbol_Table table;
    inject_builtins(table);
    auto debug_val = table.lookup("debug_dump");
    REQUIRE(debug_val->is<Function>());
    auto debug_dump = debug_val->get<Function>().value();

    SECTION("Injected")
    {
        CHECK(debug_val->is<Function>());
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(debug_dump->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(debug_dump->call({Value::create(), Value::create()}),
                          ContainsSubstring("too many arguments"));
    }

    SECTION("Non-function values use to_string")
    {
        std::vector<Value_Ptr> values{
            Value::create(), // Null
            Value::create(42_f),
            Value::create(3.14),
            Value::create(true),
            Value::create("hello"s),
            Value::create(frst::Array{Value::create(1_f)}),
            Value::create(frst::Map{
                {Value::create("k"s), Value::create(1_f)},
            }),
        };

        for (const auto& val : values)
        {
            auto res = debug_dump->call({val});
            REQUIRE(res->is<frst::String>());
            CHECK(res->get<frst::String>().value()
                  == val->to_internal_string());
        }
    }

    SECTION("Builtin function debug_dump")
    {
        auto func = Value::create(Function{std::make_shared<Builtin>(
            [](builtin_args_t) { return Value::create(); }, "dbg",
            Builtin::Arity{0, 0})});

        auto res = debug_dump->call({func});
        REQUIRE(res->is<frst::String>());
        CHECK(res->get<frst::String>().value() == "<builtin:dbg>");
    }

    SECTION("Custom callable debug_dump")
    {
        struct Dummy final : Callable
        {
            Value_Ptr call(builtin_args_t) const override
            {
                return Value::create();
            }
            std::string debug_dump() const override
            {
                return "<dummy>";
            }
        };

        auto func = Value::create(Function{std::make_shared<Dummy>()});

        auto res = debug_dump->call({func});
        REQUIRE(res->is<frst::String>());
        CHECK(res->get<frst::String>().value() == "<dummy>");
    }
}
