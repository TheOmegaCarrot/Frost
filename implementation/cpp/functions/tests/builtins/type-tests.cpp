#include <catch2/catch_all.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <frost/builtin.hpp>

using namespace frst;

using namespace std::literals;

TEST_CASE("Type Tests")
{
    // AI-generated test additions by Codex (GPT-5).
    // Signed: Codex (GPT-5).

    Symbol_Table table;
    inject_builtins(table);

    auto get_fn = [&](const std::string& name) {
        auto val = table.lookup(name);
        REQUIRE(val->is<Function>());
        return val->get<Function>().value();
    };

    auto call_bool = [&](const std::string& name, const Value_Ptr& arg) {
        auto fn = get_fn(name);
        auto res = fn->call({arg});
        REQUIRE(res->is<Bool>());
        return res->get<Bool>().value_or(false);
    };

    struct Dummy final : Callable
    {
        Value_Ptr call(builtin_args_t) const override
        {
            return Value::null();
        }
        std::string debug_dump() const override
        {
            return "<dummy>";
        }
    };

    auto v_null = Value::null();
    auto v_int = Value::create(42_f);
    auto v_float = Value::create(3.14);
    auto v_bool = Value::create(true);
    auto v_string = Value::create("hello"s);
    auto v_array = Value::create(frst::Array{Value::create(1_f)});
    auto v_map =
        Value::create(frst::Map{{Value::create("k"s), Value::create(1_f)}});
    auto v_func = Value::create(Function{std::make_shared<Dummy>()});

    SECTION("Injected")
    {
        const std::vector<std::string> names{
            "is_null",    "is_int",     "is_float",     "is_bool",
            "is_string",  "is_array",   "is_map",       "is_function",
            "is_nonnull", "is_numeric", "is_primitive", "is_structured",
        };

        for (const auto& name : names)
        {
            auto val = table.lookup(name);
            REQUIRE(val->is<Function>());
        }
    }

    SECTION("is_null")
    {
        CHECK(call_bool("is_null", v_null));
        CHECK_FALSE(call_bool("is_null", v_int));
    }

    SECTION("is_int")
    {
        CHECK(call_bool("is_int", v_int));
        CHECK_FALSE(call_bool("is_int", v_float));
    }

    SECTION("is_float")
    {
        CHECK(call_bool("is_float", v_float));
        CHECK_FALSE(call_bool("is_float", v_int));
    }

    SECTION("is_bool")
    {
        CHECK(call_bool("is_bool", v_bool));
        CHECK_FALSE(call_bool("is_bool", v_string));
    }

    SECTION("is_string")
    {
        CHECK(call_bool("is_string", v_string));
        CHECK_FALSE(call_bool("is_string", v_array));
    }

    SECTION("is_array")
    {
        CHECK(call_bool("is_array", v_array));
        CHECK_FALSE(call_bool("is_array", v_map));
    }

    SECTION("is_map")
    {
        CHECK(call_bool("is_map", v_map));
        CHECK_FALSE(call_bool("is_map", v_array));
    }

    SECTION("is_function")
    {
        CHECK(call_bool("is_function", v_func));
        CHECK_FALSE(call_bool("is_function", v_null));
    }

    SECTION("is_nonnull")
    {
        CHECK_FALSE(call_bool("is_nonnull", v_null));
        CHECK(call_bool("is_nonnull", v_int));
        CHECK(call_bool("is_nonnull", v_float));
        CHECK(call_bool("is_nonnull", v_bool));
        CHECK(call_bool("is_nonnull", v_string));
        CHECK(call_bool("is_nonnull", v_array));
        CHECK(call_bool("is_nonnull", v_map));
        CHECK(call_bool("is_nonnull", v_func));
    }

    SECTION("is_numeric")
    {
        CHECK(call_bool("is_numeric", v_int));
        CHECK(call_bool("is_numeric", v_float));
        CHECK_FALSE(call_bool("is_numeric", v_null));
        CHECK_FALSE(call_bool("is_numeric", v_bool));
        CHECK_FALSE(call_bool("is_numeric", v_string));
        CHECK_FALSE(call_bool("is_numeric", v_array));
        CHECK_FALSE(call_bool("is_numeric", v_map));
        CHECK_FALSE(call_bool("is_numeric", v_func));
    }

    SECTION("is_primitive")
    {
        CHECK(call_bool("is_primitive", v_null));
        CHECK(call_bool("is_primitive", v_int));
        CHECK(call_bool("is_primitive", v_float));
        CHECK(call_bool("is_primitive", v_bool));
        CHECK(call_bool("is_primitive", v_string));
        CHECK_FALSE(call_bool("is_primitive", v_array));
        CHECK_FALSE(call_bool("is_primitive", v_map));
        CHECK_FALSE(call_bool("is_primitive", v_func));
    }

    SECTION("is_structured")
    {
        CHECK(call_bool("is_structured", v_array));
        CHECK(call_bool("is_structured", v_map));
        CHECK_FALSE(call_bool("is_structured", v_null));
        CHECK_FALSE(call_bool("is_structured", v_int));
        CHECK_FALSE(call_bool("is_structured", v_float));
        CHECK_FALSE(call_bool("is_structured", v_bool));
        CHECK_FALSE(call_bool("is_structured", v_string));
        CHECK_FALSE(call_bool("is_structured", v_func));
    }
}
