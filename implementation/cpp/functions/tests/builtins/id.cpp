#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/testing/dummy-callable.hpp>
#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <frost/builtin.hpp>

using namespace frst;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;

TEST_CASE("Builtin id")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).

    Symbol_Table table;
    inject_builtins(table);
    auto id_val = table.lookup("id");
    REQUIRE(id_val->is<Function>());
    auto id = id_val->get<Function>().value();

    SECTION("Injected")
    {
        CHECK(id_val->is<Function>());
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(id->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(id->call({Value::null(), Value::null()}),
                          ContainsSubstring("too many arguments"));
    }

    SECTION("Passes through values by pointer")
    {
        auto v_null = Value::null();
        auto v_int = Value::create(42_f);
        auto v_float = Value::create(3.14);
        auto v_bool = Value::create(true);
        auto v_str = Value::create("hello"s);
        auto v_arr = Value::create(Array{Value::create(1_f)});
        auto v_map = Value::create(Map{{Value::create("k"s),
                                        Value::create(2_f)}});
        auto v_fn = Value::create(Function{
            std::make_shared<frst::testing::Dummy_Callable>()});

        CHECK(id->call({v_null}) == v_null);
        CHECK(id->call({v_int}) == v_int);
        CHECK(id->call({v_float}) == v_float);
        CHECK(id->call({v_bool}) == v_bool);
        CHECK(id->call({v_str}) == v_str);
        CHECK(id->call({v_arr}) == v_arr);
        CHECK(id->call({v_map}) == v_map);
        CHECK(id->call({v_fn}) == v_fn);
    }
}
