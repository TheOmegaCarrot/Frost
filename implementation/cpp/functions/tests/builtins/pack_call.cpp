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
using trompeloeil::_;

TEST_CASE("Builtin pack_call")
{
    // AI-generated test additions by Codex (GPT-5).
    // Signed: Codex (GPT-5).

    Symbol_Table table;
    inject_builtins(table);

    auto pack_call_val = table.lookup("pack_call");
    REQUIRE(pack_call_val->is<Function>());
    auto pack_call_fn = pack_call_val->get<Function>().value();

    SECTION("Arity: too few arguments")
    {
        CHECK_THROWS_WITH(pack_call_fn->call({}),
                          ContainsSubstring("insufficient arguments"));
    }

    SECTION("Arity: too many arguments")
    {
        CHECK_THROWS_WITH(
            pack_call_fn->call({Value::null(), Value::null(), Value::null()}),
            ContainsSubstring("too many arguments"));
    }

    SECTION("Type error: function argument")
    {
        auto arg_list = Value::create(frst::Array{Value::create(1_f)});
        try
        {
            pack_call_fn->call({Value::create(1_f), arg_list});
            FAIL("Expected type error for function argument");
        }
        catch (const Frost_User_Error& err)
        {
            const std::string msg = err.what();
            CHECK_THAT(msg, ContainsSubstring("Function pack_call"));
            CHECK_THAT(msg,
                       ContainsSubstring("requires Function as argument 1"));
            CHECK_THAT(msg, ContainsSubstring("(function)"));
            CHECK_THAT(msg, ContainsSubstring("got Int"));
        }
    }

    SECTION("Type error: args argument")
    {
        auto callable = mock::Mock_Callable::make();
        auto func_val = Value::create(Function{callable});
        try
        {
            pack_call_fn->call({func_val, Value::create(1_f)});
            FAIL("Expected type error for args argument");
        }
        catch (const Frost_User_Error& err)
        {
            const std::string msg = err.what();
            CHECK_THAT(msg, ContainsSubstring("Function pack_call"));
            CHECK_THAT(msg, ContainsSubstring("requires Array as argument 2"));
            CHECK_THAT(msg, ContainsSubstring("(args)"));
            CHECK_THAT(msg, ContainsSubstring("got Int"));
        }
    }

    SECTION("Empty array calls zero-arg function")
    {
        auto callable = mock::Mock_Callable::make();
        auto func_val = Value::create(Function{callable});
        auto empty_args = Value::create(frst::Array{});

        bool called = false;
        std::size_t observed_size = 0;
        REQUIRE_CALL(*callable, call(_))
            .LR_SIDE_EFFECT({
                called = true;
                observed_size = _1.size();
            })
            .RETURN(Value::null());

        auto res = pack_call_fn->call({func_val, empty_args});

        CHECK(called);
        CHECK(observed_size == 0);
        CHECK(res->is<Null>());
    }

    SECTION("Passes through array without copying")
    {
        auto arr_val = Value::create(frst::Array{
            Value::create(1_f),
            Value::create("two"s),
            Value::create(true),
        });
        const auto* expected = &arr_val->raw_get<Array>();

        auto callable = mock::Mock_Callable::make();
        auto expected_return = Value::create("ok"s);
        bool called = false;
        bool same_vector = false;
        bool same_elements = false;
        std::size_t observed_size = 0;

        auto func_val = Value::create(Function{callable});
        REQUIRE_CALL(*callable, call(_))
            .LR_SIDE_EFFECT({
                called = true;
                observed_size = _1.size();
                same_vector = (_1.data() == expected->data());
                same_elements = (_1.size() == expected->size());
                if (same_elements)
                {
                    for (std::size_t i = 0; i < _1.size(); ++i)
                    {
                        if (_1[i] != expected->at(i))
                        {
                            same_elements = false;
                            break;
                        }
                    }
                }
            })
            .RETURN(expected_return);
        auto res = pack_call_fn->call({func_val, arr_val});

        CHECK(called);
        CHECK(observed_size == expected->size());
        CHECK(same_vector);
        CHECK(same_elements);
        CHECK(res == expected_return);
    }

    SECTION("Propagates errors from callee")
    {
        auto callable = mock::Mock_Callable::make();
        auto func_val = Value::create(Function{callable});
        auto args = Value::create(frst::Array{Value::create(1_f)});

        REQUIRE_CALL(*callable, call(_)).THROW(Frost_User_Error{"boom"});
        CHECK_THROWS_WITH(pack_call_fn->call({func_val, args}),
                          ContainsSubstring("boom"));
    }

    SECTION("Callee arity errors propagate")
    {
        auto callee = std::make_shared<Builtin>(
            [](builtin_args_t) {
                return Value::null();
            },
            "exact_arity_0", Builtin::Arity{0, 0});
        auto func_val = Value::create(Function{callee});
        auto args = Value::create(frst::Array{
            Value::create(42_f),
            Value::create(10_f),
        });

        try
        {
            pack_call_fn->call({func_val, args});
            FAIL("Expected arity error from callee");
        }
        catch (const Frost_User_Error& err)
        {
            const std::string msg = err.what();
            CHECK_THAT(msg, ContainsSubstring("Function exact_arity_0"));
            CHECK_THAT(msg, ContainsSubstring("too many arguments"));
        }
    }
}
