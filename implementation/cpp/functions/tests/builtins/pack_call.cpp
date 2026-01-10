#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <frost/builtin.hpp>

using namespace frst;

using namespace std::literals;
using Catch::Matchers::ContainsSubstring;

TEST_CASE("pack_call")
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
        CHECK_THROWS_WITH(pack_call_fn->call({Value::create(), Value::create(),
                                              Value::create()}),
                          ContainsSubstring("too many arguments"));
    }

    struct RecordingCallable final : Callable
    {
        const Array* expected_args = nullptr;
        mutable bool called = false;
        mutable bool same_vector = false;
        mutable bool same_elements = false;
        Value_Ptr return_value;
        mutable std::size_t observed_size = 0;

        Value_Ptr call(builtin_args_t args) const override
        {
            called = true;
            observed_size = args.size();
            if (expected_args != nullptr)
            {
                same_vector = (&args == expected_args);
                same_elements = (args.size() == expected_args->size());
                if (same_elements)
                {
                    for (std::size_t i = 0; i < args.size(); ++i)
                    {
                        if (args[i] != expected_args->at(i))
                        {
                            same_elements = false;
                            break;
                        }
                    }
                }
            }
            return return_value ? return_value : Value::create();
        }

        std::string debug_dump() const override
        {
            return "<recording>";
        }
    };

    struct ThrowingCallable final : Callable
    {
        Value_Ptr call(builtin_args_t) const override
        {
            throw Frost_Error{"boom"};
        }
        std::string debug_dump() const override
        {
            return "<throwing>";
        }
    };

    SECTION("Type error: function argument")
    {
        auto arg_list = Value::create(frst::Array{Value::create(1_f)});
        try
        {
            pack_call_fn->call({Value::create(1_f), arg_list});
            FAIL("Expected type error for function argument");
        }
        catch (const Frost_Error& err)
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
        auto callable = std::make_shared<RecordingCallable>();
        auto func_val = Value::create(Function{callable});
        try
        {
            pack_call_fn->call({func_val, Value::create(1_f)});
            FAIL("Expected type error for args argument");
        }
        catch (const Frost_Error& err)
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
        auto callable = std::make_shared<RecordingCallable>();
        auto func_val = Value::create(Function{callable});
        auto empty_args = Value::create(frst::Array{});

        auto res = pack_call_fn->call({func_val, empty_args});

        CHECK(callable->called);
        CHECK(callable->observed_size == 0);
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

        auto callable = std::make_shared<RecordingCallable>();
        callable->expected_args = expected;
        auto expected_return = Value::create("ok"s);
        callable->return_value = expected_return;

        auto func_val = Value::create(Function{callable});
        auto res = pack_call_fn->call({func_val, arr_val});

        CHECK(callable->called);
        CHECK(callable->same_vector);
        CHECK(callable->same_elements);
        CHECK(res == expected_return);
    }

    SECTION("Propagates errors from callee")
    {
        auto callable = std::make_shared<ThrowingCallable>();
        auto func_val = Value::create(Function{callable});
        auto args = Value::create(frst::Array{Value::create(1_f)});

        CHECK_THROWS_WITH(pack_call_fn->call({func_val, args}),
                          ContainsSubstring("boom"));
    }

    SECTION("Callee arity errors propagate")
    {
        auto callee = std::make_shared<Builtin>(
            [](builtin_args_t) { return Value::create(); }, "exact_arity_0",
            Builtin::Arity{0, 0});
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
        catch (const Frost_Error& err)
        {
            const std::string msg = err.what();
            CHECK_THAT(msg, ContainsSubstring("Function exact_arity_0"));
            CHECK_THAT(msg, ContainsSubstring("too many arguments"));
        }
    }
}
