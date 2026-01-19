#include <catch2/catch_all.hpp>

#include <cmath>

#include <frost/testing/dummy-callable.hpp>
#include <frost/value.hpp>

using namespace std::literals;
using namespace frst::literals;
using frst::Value, frst::Value_Ptr;

namespace
{
// AI-generated test additions by Codex (GPT-5).
// Signed: Codex (GPT-5).
using frst::testing::Dummy_Callable;
} // namespace

TEST_CASE("to_string")
{
    auto Null = Value::null();
    auto Int = Value::create(42_f);
    auto Float = Value::create(3.14);
    auto Bool = Value::create(true);
    auto String = Value::create("Hello!"s);
    auto Array = Value::create(frst::Array{
        Value::create(42_f),
        Value::create("hello"s),
    });
    auto Map = Value::create(frst::Map{
        {Value::create("key1"s), Value::create(42_f)},
        {Value::create(true), Value::create("value2"s)},
    });
    auto Function =
        Value::create(frst::Function{std::make_shared<Dummy_Callable>()});

    auto string_with_quotes = Value::create("hi \" there"s);

    CHECK(Null->to_internal_string() == "null");
    CHECK(Int->to_internal_string() == "42");
    CHECK(Float->to_internal_string().starts_with("3.14"));
    CHECK(Bool->to_internal_string() == "true");
    CHECK(String->to_internal_string() == "Hello!");
    CHECK(Array->to_internal_string() == R"([ 42, "hello" ])");
    CHECK(Map->to_internal_string() == R"({ [true]: "value2", ["key1"]: 42 })");
    CHECK(Function->to_internal_string() == "<Function>");

    auto Nested = Value::create(frst::Array{
        Value::create(42_f),
        Array,
    });

    CHECK(Nested->to_internal_string() == R"([ 42, [ 42, "hello" ] ])");

    CHECK(string_with_quotes->to_internal_string() == R"(hi " there)");
}

// AI-generated test additions by Codex (GPT-5).
// Signed: Codex (GPT-5).

TEST_CASE("to_string Value_Ptr")
{
    auto Int = Value::create(42_f);
    auto Bool = Value::create(true);
    auto Function =
        Value::create(frst::Function{std::make_shared<Dummy_Callable>()});

    auto int_str = Int->to_string();
    REQUIRE(int_str->is<frst::String>());
    CHECK(int_str->get<frst::String>().value() == "42");

    auto bool_str = Bool->to_string();
    REQUIRE(bool_str->is<frst::String>());
    CHECK(bool_str->get<frst::String>().value() == "true");

    auto fn_str = Function->to_string();
    REQUIRE(fn_str->is<frst::String>());
    CHECK(fn_str->get<frst::String>().value() == "<Function>");
}

TEST_CASE("to_internal_int")
{
    auto v_null = Value::null();
    auto v_int = Value::create(42_f);
    auto v_float = Value::create(3.9);
    auto v_float_neg = Value::create(-3.9);
    auto v_bool = Value::create(true);
    auto v_string = Value::create("123"s);
    auto v_string_plus = Value::create("+17"s);
    auto v_string_neg = Value::create("-5"s);
    auto v_string_leading_zero = Value::create("003"s);
    auto v_string_bad = Value::create("42abc"s);
    auto v_string_empty = Value::create(""s);
    auto v_string_hex = Value::create("0x10"s);
    auto v_string_exp = Value::create("1e3"s);
    auto v_string_ws_lead = Value::create(" 1"s);
    auto v_string_ws_trail = Value::create("1 "s);
    auto v_string_float = Value::create("3.14"s);
    auto v_string_overflow = Value::create("9223372036854775808"s);
    auto v_string_underflow = Value::create("-9223372036854775809"s);
    auto v_array = Value::create(frst::Array{Value::create(1_f)});
    auto v_map =
        Value::create(frst::Map{{Value::create("k"s), Value::create(1_f)}});
    auto v_func =
        Value::create(frst::Function{std::make_shared<Dummy_Callable>()});

    CHECK(v_int->to_internal_int() == 42);
    CHECK(v_float->to_internal_int() == 3);
    CHECK(v_float_neg->to_internal_int() == -3);
    CHECK(v_string->to_internal_int() == 123);
    CHECK(v_string_neg->to_internal_int() == -5);
    CHECK(v_string_leading_zero->to_internal_int() == 3);

    CHECK_FALSE(v_null->to_internal_int().has_value());
    CHECK_FALSE(v_bool->to_internal_int().has_value());
    CHECK_FALSE(v_array->to_internal_int().has_value());
    CHECK_FALSE(v_map->to_internal_int().has_value());
    CHECK_FALSE(v_func->to_internal_int().has_value());
    CHECK_FALSE(v_string_bad->to_internal_int().has_value());
    CHECK_FALSE(v_string_empty->to_internal_int().has_value());
    CHECK_FALSE(v_string_hex->to_internal_int().has_value());
    CHECK_FALSE(v_string_exp->to_internal_int().has_value());
    CHECK_FALSE(v_string_plus->to_internal_int().has_value());
    CHECK_FALSE(v_string_ws_lead->to_internal_int().has_value());
    CHECK_FALSE(v_string_ws_trail->to_internal_int().has_value());
    CHECK_FALSE(v_string_float->to_internal_int().has_value());
    CHECK_FALSE(v_string_overflow->to_internal_int().has_value());
    CHECK_FALSE(v_string_underflow->to_internal_int().has_value());
}

TEST_CASE("to_int")
{
    auto v_int = Value::create(42_f);
    auto v_float = Value::create(3.9);
    auto v_string = Value::create("123"s);
    auto v_string_plus = Value::create("+17"s);
    auto v_string_neg = Value::create("-5"s);
    auto v_string_float = Value::create("3.14"s);
    auto v_string_hex = Value::create("0x10"s);
    auto v_string_exp = Value::create("1e3"s);
    auto v_string_overflow = Value::create("9223372036854775808"s);
    auto v_string_underflow = Value::create("-9223372036854775809"s);
    auto v_bool = Value::create(true);
    auto v_null = Value::null();
    auto v_array = Value::create(frst::Array{Value::create(1_f)});
    auto v_map =
        Value::create(frst::Map{{Value::create("k"s), Value::create(1_f)}});
    auto v_func =
        Value::create(frst::Function{std::make_shared<Dummy_Callable>()});

    auto res_int = v_int->to_int();
    REQUIRE(res_int->is<frst::Int>());
    CHECK(res_int->get<frst::Int>().value() == 42);

    auto res_float = v_float->to_int();
    REQUIRE(res_float->is<frst::Int>());
    CHECK(res_float->get<frst::Int>().value() == 3);

    auto res_string = v_string->to_int();
    REQUIRE(res_string->is<frst::Int>());
    CHECK(res_string->get<frst::Int>().value() == 123);

    auto res_string_neg = v_string_neg->to_int();
    REQUIRE(res_string_neg->is<frst::Int>());
    CHECK(res_string_neg->get<frst::Int>().value() == -5);

    CHECK(v_string_float->to_int()->is<frst::Null>());
    CHECK(v_string_hex->to_int()->is<frst::Null>());
    CHECK(v_string_exp->to_int()->is<frst::Null>());
    CHECK(v_string_plus->to_int()->is<frst::Null>());
    CHECK(v_string_overflow->to_int()->is<frst::Null>());
    CHECK(v_string_underflow->to_int()->is<frst::Null>());
    CHECK(v_bool->to_int()->is<frst::Null>());
    CHECK(v_null->to_int()->is<frst::Null>());
    CHECK(v_array->to_int()->is<frst::Null>());
    CHECK(v_map->to_int()->is<frst::Null>());
    CHECK(v_func->to_int()->is<frst::Null>());
}

TEST_CASE("to_internal_float")
{
    auto v_null = Value::null();
    auto v_int = Value::create(42_f);
    auto v_float = Value::create(3.14);
    auto v_bool = Value::create(true);
    auto v_string = Value::create("3.14"s);
    auto v_string_plus = Value::create("+2.5"s);
    auto v_string_neg = Value::create("-0.25"s);
    auto v_string_int = Value::create("42"s);
    auto v_string_bad = Value::create("3.14.15"s);
    auto v_string_empty = Value::create(""s);
    auto v_string_hex = Value::create("0x10"s);
    auto v_string_exp = Value::create("1e3"s);
    auto v_string_overflow = Value::create(std::string(400, '9'));
    auto v_string_overflow_neg = Value::create("-" + std::string(400, '9'));
    auto v_string_ws_lead = Value::create(" 1"s);
    auto v_string_ws_trail = Value::create("1 "s);
    auto v_array = Value::create(frst::Array{Value::create(1_f)});
    auto v_map =
        Value::create(frst::Map{{Value::create("k"s), Value::create(1_f)}});
    auto v_func =
        Value::create(frst::Function{std::make_shared<Dummy_Callable>()});

    CHECK(v_int->to_internal_float() == 42.0);
    CHECK(v_float->to_internal_float() == 3.14);
    CHECK(v_string->to_internal_float() == 3.14);
    CHECK(v_string_neg->to_internal_float() == -0.25);
    CHECK(v_string_int->to_internal_float() == 42.0);

    CHECK_FALSE(v_null->to_internal_float().has_value());
    CHECK_FALSE(v_bool->to_internal_float().has_value());
    CHECK_FALSE(v_array->to_internal_float().has_value());
    CHECK_FALSE(v_map->to_internal_float().has_value());
    CHECK_FALSE(v_func->to_internal_float().has_value());
    CHECK_FALSE(v_string_bad->to_internal_float().has_value());
    CHECK_FALSE(v_string_empty->to_internal_float().has_value());
    CHECK_FALSE(v_string_hex->to_internal_float().has_value());
    CHECK(v_string_exp->to_internal_float() == 1000.0);
    CHECK_FALSE(v_string_plus->to_internal_float().has_value());
    CHECK_FALSE(v_string_ws_lead->to_internal_float().has_value());
    CHECK_FALSE(v_string_ws_trail->to_internal_float().has_value());
    CHECK_FALSE(v_string_overflow->to_internal_float().has_value());
    CHECK_FALSE(v_string_overflow_neg->to_internal_float().has_value());
}

TEST_CASE("to_float")
{
    auto v_int = Value::create(42_f);
    auto v_float = Value::create(3.14);
    auto v_string = Value::create("3.14"s);
    auto v_string_plus = Value::create("+2.5"s);
    auto v_string_neg = Value::create("-0.25"s);
    auto v_string_int = Value::create("42"s);
    auto v_string_bad = Value::create("3.14.15"s);
    auto v_string_empty = Value::create(""s);
    auto v_string_hex = Value::create("0x10"s);
    auto v_string_exp = Value::create("1e3"s);
    auto v_string_overflow = Value::create(std::string(400, '9'));
    auto v_string_overflow_neg = Value::create("-" + std::string(400, '9'));
    auto v_bool = Value::create(true);
    auto v_null = Value::null();
    auto v_array = Value::create(frst::Array{Value::create(1_f)});
    auto v_map =
        Value::create(frst::Map{{Value::create("k"s), Value::create(1_f)}});
    auto v_func =
        Value::create(frst::Function{std::make_shared<Dummy_Callable>()});

    auto res_int = v_int->to_float();
    REQUIRE(res_int->is<frst::Float>());
    CHECK(res_int->get<frst::Float>().value() == 42.0);

    auto res_float = v_float->to_float();
    REQUIRE(res_float->is<frst::Float>());
    CHECK(res_float->get<frst::Float>().value() == 3.14);

    auto res_string = v_string->to_float();
    REQUIRE(res_string->is<frst::Float>());
    CHECK(res_string->get<frst::Float>().value() == 3.14);

    auto res_string_neg = v_string_neg->to_float();
    REQUIRE(res_string_neg->is<frst::Float>());
    CHECK(res_string_neg->get<frst::Float>().value() == -0.25);

    auto res_string_int = v_string_int->to_float();
    REQUIRE(res_string_int->is<frst::Float>());
    CHECK(res_string_int->get<frst::Float>().value() == 42.0);

    CHECK(v_string_bad->to_float()->is<frst::Null>());
    CHECK(v_string_empty->to_float()->is<frst::Null>());
    CHECK(v_string_hex->to_float()->is<frst::Null>());
    auto res_string_exp = v_string_exp->to_float();
    REQUIRE(res_string_exp->is<frst::Float>());
    CHECK(res_string_exp->get<frst::Float>().value() == 1000.0);
    CHECK(v_string_plus->to_float()->is<frst::Null>());
    CHECK(v_string_overflow->to_float()->is<frst::Null>());
    CHECK(v_string_overflow_neg->to_float()->is<frst::Null>());
    CHECK(v_bool->to_float()->is<frst::Null>());
    CHECK(v_null->to_float()->is<frst::Null>());
    CHECK(v_array->to_float()->is<frst::Null>());
    CHECK(v_map->to_float()->is<frst::Null>());
    CHECK(v_func->to_float()->is<frst::Null>());
}
