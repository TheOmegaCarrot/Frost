// AI-generated test by Codex (GPT-5).
// Signed: Codex (GPT-5).
#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>
#include <catch2/trompeloeil.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/mock/mock-callable.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <frost/builtin.hpp>

using namespace frst;

using namespace std::literals;
using Catch::Matchers::ContainsSubstring;
using Catch::Matchers::EndsWith;
using trompeloeil::_;

TEST_CASE("Builtin and_then/or_else")
{
    Symbol_Table table;
    inject_builtins(table);

    auto and_then_val = table.lookup("and_then");
    auto or_else_val = table.lookup("or_else");
    REQUIRE(and_then_val->is<Function>());
    REQUIRE(or_else_val->is<Function>());

    auto and_then_fn = and_then_val->get<Function>().value();
    auto or_else_fn = or_else_val->get<Function>().value();

    SECTION("Injected")
    {
        CHECK(and_then_val->is<Function>());
        CHECK(or_else_val->is<Function>());
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(and_then_fn->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(
            and_then_fn->call({Value::null(), Value::null(), Value::null()}),
            ContainsSubstring("too many arguments"));

        CHECK_THROWS_WITH(or_else_fn->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(
            or_else_fn->call({Value::null(), Value::null(), Value::null()}),
            ContainsSubstring("too many arguments"));
    }

    SECTION("Type errors")
    {
        auto bad = Value::create(1_f);
        auto good = Value::null();

        CHECK_THROWS_WITH(and_then_fn->call({good, bad}),
                          ContainsSubstring("Function and_then")
                              && ContainsSubstring("Function")
                              && ContainsSubstring("argument 2")
                              && EndsWith(std::string{bad->type_name()}));

        CHECK_THROWS_WITH(or_else_fn->call({good, bad}),
                          ContainsSubstring("Function or_else")
                              && ContainsSubstring("Function")
                              && ContainsSubstring("argument 2")
                              && EndsWith(std::string{bad->type_name()}));
    }

    SECTION("and_then with null returns null and skips call")
    {
        auto callable = mock::Mock_Callable::make();
        auto fn_val = Value::create(Function{callable});

        FORBID_CALL(*callable, call(_));

        auto res = and_then_fn->call({Value::null(), fn_val});
        CHECK(res == Value::null());
    }

    SECTION("and_then calls function with arg when non-null")
    {
        auto callable = mock::Mock_Callable::make();
        auto fn_val = Value::create(Function{callable});
        auto arg = Value::create(42_f);
        auto expected = Value::create("ok"s);

        bool called = false;
        bool arg_ok = false;
        REQUIRE_CALL(*callable, call(_))
            .LR_SIDE_EFFECT({
                called = true;
                arg_ok = (_1.size() == 1 && _1.at(0) == arg);
            })
            .RETURN(expected);

        auto res = and_then_fn->call({arg, fn_val});
        CHECK(called);
        CHECK(arg_ok);
        CHECK(res == expected);
    }

    SECTION("or_else returns arg and skips call when non-null")
    {
        auto callable = mock::Mock_Callable::make();
        auto fn_val = Value::create(Function{callable});
        auto arg = Value::create(42_f);

        FORBID_CALL(*callable, call(_));

        auto res = or_else_fn->call({arg, fn_val});
        CHECK(res == arg);
    }

    SECTION("or_else calls function when null")
    {
        auto callable = mock::Mock_Callable::make();
        auto fn_val = Value::create(Function{callable});
        auto expected = Value::create("ok"s);

        bool called = false;
        std::size_t observed_size = 0;
        REQUIRE_CALL(*callable, call(_))
            .LR_SIDE_EFFECT({
                called = true;
                observed_size = _1.size();
            })
            .RETURN(expected);

        auto res = or_else_fn->call({Value::null(), fn_val});
        CHECK(called);
        CHECK(observed_size == 0);
        CHECK(res == expected);
    }
}
