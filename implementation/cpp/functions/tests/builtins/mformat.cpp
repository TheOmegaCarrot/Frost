#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <frost/builtin.hpp>

using namespace frst;

using namespace std::literals;

using namespace Catch::Matchers;

TEST_CASE("Builtin mformat")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    Symbol_Table table;
    inject_builtins(table);
    auto mformat_val = table.lookup("mformat");
    REQUIRE(mformat_val->is<Function>());
    auto mformat = mformat_val->get<Function>().value();

    SECTION("Injected")
    {
        CHECK(mformat_val->is<Function>());
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(mformat->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(mformat->call({}),
                          ContainsSubstring("requires at least 2"));

        CHECK_THROWS_WITH(mformat->call({Value::null(), Value::null(),
                                         Value::null()}),
                          ContainsSubstring("too many arguments"));
        CHECK_THROWS_WITH(mformat->call({Value::null(), Value::null(),
                                         Value::null()}),
                          ContainsSubstring("no more than 2"));
    }

    SECTION("Type checks")
    {
        SECTION("First argument must be String")
        {
            auto bad_fmt = Value::create(42_f);
            auto repl = Value::create(Map{});
            CHECK_THROWS_WITH(mformat->call({bad_fmt, repl}),
                              ContainsSubstring("Function mformat requires"));
            CHECK_THROWS_WITH(mformat->call({bad_fmt, repl}),
                              ContainsSubstring("format string"));
            CHECK_THROWS_WITH(mformat->call({bad_fmt, repl}),
                              EndsWith(std::string{bad_fmt->type_name()}));
        }

        SECTION("Second argument must be Map")
        {
            auto fmt = Value::create("${k}"s);
            auto bad_repl = Value::create("nope"s);
            CHECK_THROWS_WITH(mformat->call({fmt, bad_repl}),
                              ContainsSubstring("Function mformat requires"));
            CHECK_THROWS_WITH(mformat->call({fmt, bad_repl}),
                              ContainsSubstring("replacement map"));
            CHECK_THROWS_WITH(mformat->call({fmt, bad_repl}),
                              EndsWith(std::string{bad_repl->type_name()}));
        }
    }

    SECTION("Success")
    {
        SECTION("Replaces placeholders with map values")
        {
            auto fmt = Value::create("${k1} ${k2}"s);
            auto repl = Value::create(Map{
                {Value::create("k1"s), Value::create("hello"s)},
                {Value::create("k2"s), Value::create("world!"s)},
            });

            auto res = mformat->call({fmt, repl});
            REQUIRE(res->is<String>());
            CHECK(res->get<String>().value() == "hello world!");
        }

        SECTION("Non-string values are converted to strings")
        {
            auto fmt = Value::create("x${n}y"s);
            auto repl = Value::create(Map{
                {Value::create("n"s), Value::create(42_f)},
            });

            auto res = mformat->call({fmt, repl});
            REQUIRE(res->is<String>());
            CHECK(res->get<String>().value() == "x42y");
        }
    }

    SECTION("Errors")
    {
        SECTION("Missing key errors")
        {
            auto fmt = Value::create("${k1} ${k2}"s);
            auto repl = Value::create(Map{
                {Value::create("k1"s), Value::create("hello"s)},
            });

            CHECK_THROWS_WITH(mformat->call({fmt, repl}),
                              ContainsSubstring("Missing replacement for key"));
            CHECK_THROWS_WITH(mformat->call({fmt, repl}),
                              ContainsSubstring("k2"));
        }

        SECTION("Null replacement value errors")
        {
            auto fmt = Value::create("${k1}"s);
            auto repl = Value::create(Map{
                {Value::create("k1"s), Value::null()},
            });

            CHECK_THROWS_WITH(mformat->call({fmt, repl}),
                              ContainsSubstring("Replacement value"));
            CHECK_THROWS_WITH(mformat->call({fmt, repl}),
                              ContainsSubstring("k1"));
        }

        SECTION("Unterminated placeholder errors")
        {
            auto fmt = Value::create("${k1"s);
            auto repl = Value::create(Map{
                {Value::create("k1"s), Value::create("x"s)},
            });

            CHECK_THROWS_WITH(
                mformat->call({fmt, repl}),
                ContainsSubstring("Unterminated format placeholder"));
        }

        SECTION("Empty placeholder errors")
        {
            auto fmt = Value::create("${}"s);
            auto repl = Value::create(Map{});

            CHECK_THROWS_WITH(mformat->call({fmt, repl}),
                              ContainsSubstring("Empty format placeholder"));
        }

        SECTION("Invalid placeholder identifier errors")
        {
            SECTION("Starts with a digit")
            {
                auto fmt = Value::create("${1abc}"s);
                auto repl = Value::create(Map{
                    {Value::create("1abc"s), Value::create("x"s)},
                });

                CHECK_THROWS_WITH(
                    mformat->call({fmt, repl}),
                    ContainsSubstring("Invalid format placeholder"));
                CHECK_THROWS_WITH(mformat->call({fmt, repl}),
                                  ContainsSubstring("1abc"));
            }

            SECTION("Contains non-identifier characters")
            {
                auto fmt = Value::create("${a-b}"s);
                auto repl = Value::create(Map{
                    {Value::create("a-b"s), Value::create("x"s)},
                });

                CHECK_THROWS_WITH(
                    mformat->call({fmt, repl}),
                    ContainsSubstring("Invalid format placeholder"));
                CHECK_THROWS_WITH(mformat->call({fmt, repl}),
                                  ContainsSubstring("a-b"));
            }
        }
    }
}
