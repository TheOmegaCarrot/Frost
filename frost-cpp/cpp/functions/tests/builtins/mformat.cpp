#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_exception.hpp>
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
        CHECK_THROWS_MATCHES(
            mformat->call({}), Frost_User_Error,
            MessageMatches(ContainsSubstring("insufficient arguments")
                           && ContainsSubstring("requires at least 2")));

        CHECK_THROWS_MATCHES(
            mformat->call({Value::null(), Value::null(), Value::null()}),
            Frost_User_Error,
            MessageMatches(ContainsSubstring("too many arguments")
                           && ContainsSubstring("no more than 2")));
    }

    SECTION("Type checks")
    {
        SECTION("First argument must be String")
        {
            auto bad_fmt = Value::create(42_f);
            auto repl = Value::create(Map{});
            CHECK_THROWS_MATCHES(
                mformat->call({bad_fmt, repl}), Frost_User_Error,
                MessageMatches(ContainsSubstring("Function mformat requires")
                               && ContainsSubstring("format string")
                               && EndsWith(std::string{bad_fmt->type_name()})));
        }

        SECTION("Second argument must be Map")
        {
            auto fmt = Value::create("${k}"s);
            auto bad_repl = Value::create("nope"s);
            CHECK_THROWS_MATCHES(
                mformat->call({fmt, bad_repl}), Frost_User_Error,
                MessageMatches(
                    ContainsSubstring("Function mformat requires")
                    && ContainsSubstring("replacement map")
                    && EndsWith(std::string{bad_repl->type_name()})));
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

        SECTION("No placeholders leaves string unchanged")
        {
            auto fmt = Value::create("plain text"s);
            auto repl = Value::create(Map{
                {Value::create("k"s), Value::create("v"s)},
                {Value::create(1_f), Value::create("ignored"s)},
            });

            auto res = mformat->call({fmt, repl});
            REQUIRE(res->is<String>());
            CHECK(res->get<String>().value() == "plain text");
        }

        SECTION("Literal dollar without brace is preserved")
        {
            auto fmt = Value::create("$$ and $x and price: $5"s);
            auto repl = Value::create(Map{
                {Value::create("x"s), Value::create("ignored"s)},
            });

            auto res = mformat->call({fmt, repl});
            REQUIRE(res->is<String>());
            CHECK(res->get<String>().value() == "$$ and $x and price: $5");
        }

        SECTION("Backslash escapes placeholder")
        {
            auto fmt = Value::create("\\${k}"s);
            auto repl = Value::create(Map{
                {Value::create("k"s), Value::create("v"s)},
            });

            auto res = mformat->call({fmt, repl});
            REQUIRE(res->is<String>());
            CHECK(res->get<String>().value() == "${k}");
        }

        SECTION(
            "Double backslash yields a literal backslash before a placeholder")
        {
            auto fmt = Value::create("\\\\${k}"s);
            auto repl = Value::create(Map{
                {Value::create("k"s), Value::create("v"s)},
            });

            auto res = mformat->call({fmt, repl});
            REQUIRE(res->is<String>());
            CHECK(res->get<String>().value() == "\\v");
        }

        SECTION("Dollar does not escape placeholders")
        {
            auto fmt = Value::create("$${a}"s);
            auto repl = Value::create(Map{
                {Value::create("a"s), Value::create("x"s)},
            });

            auto res = mformat->call({fmt, repl});
            REQUIRE(res->is<String>());
            CHECK(res->get<String>().value() == "$x");
        }

        SECTION("Adjacent placeholders are concatenated")
        {
            auto fmt = Value::create("${a}${b}"s);
            auto repl = Value::create(Map{
                {Value::create("a"s), Value::create("foo"s)},
                {Value::create("b"s), Value::create("bar"s)},
            });

            auto res = mformat->call({fmt, repl});
            REQUIRE(res->is<String>());
            CHECK(res->get<String>().value() == "foobar");
        }

        SECTION("Placeholders at boundaries are supported")
        {
            auto repl = Value::create(Map{
                {Value::create("a"s), Value::create("X"s)},
            });

            auto prefix = Value::create("${a}suffix"s);
            auto res_prefix = mformat->call({prefix, repl});
            REQUIRE(res_prefix->is<String>());
            CHECK(res_prefix->get<String>().value() == "Xsuffix");

            auto suffix = Value::create("prefix${a}"s);
            auto res_suffix = mformat->call({suffix, repl});
            REQUIRE(res_suffix->is<String>());
            CHECK(res_suffix->get<String>().value() == "prefixX");
        }

        SECTION("Identifier edge cases are accepted")
        {
            auto fmt = Value::create("${_} ${_x} ${a1_b2}"s);
            auto repl = Value::create(Map{
                {Value::create("_"s), Value::create("u"s)},
                {Value::create("_x"s), Value::create("ux"s)},
                {Value::create("a1_b2"s), Value::create("ab"s)},
            });

            auto res = mformat->call({fmt, repl});
            REQUIRE(res->is<String>());
            CHECK(res->get<String>().value() == "u ux ab");
        }

        SECTION("Repeated placeholders reuse the same key")
        {
            auto fmt = Value::create("${x} ${x}"s);
            auto repl = Value::create(Map{
                {Value::create("x"s), Value::create("rep"s)},
            });

            auto res = mformat->call({fmt, repl});
            REQUIRE(res->is<String>());
            CHECK(res->get<String>().value() == "rep rep");
        }

        SECTION("Unused map keys are ignored")
        {
            auto fmt = Value::create("${ok}"s);
            auto repl = Value::create(Map{
                {Value::create("ok"s), Value::create("yes"s)},
                {Value::create("not_used"s), Value::null()},
                {Value::create(1_f), Value::create("ignored"s)},
            });

            auto res = mformat->call({fmt, repl});
            REQUIRE(res->is<String>());
            CHECK(res->get<String>().value() == "yes");
        }

        SECTION("Null replacement values are formatted")
        {
            auto fmt = Value::create("${k1}"s);
            auto repl = Value::create(Map{
                {Value::create("k1"s), Value::null()},
            });

            auto res = mformat->call({fmt, repl});
            REQUIRE(res->is<String>());
            CHECK(res->get<String>().value() == "null");
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

            CHECK_THROWS_MATCHES(
                mformat->call({fmt, repl}), Frost_User_Error,
                MessageMatches(ContainsSubstring("Missing replacement for key")
                               && ContainsSubstring("k2")));
        }

        SECTION("Unterminated placeholder errors")
        {
            auto fmt = Value::create("${k1"s);
            auto repl = Value::create(Map{
                {Value::create("k1"s), Value::create("x"s)},
            });

            CHECK_THROWS_WITH(mformat->call({fmt, repl}),
                              "Unterminated format placeholder: ${k1");
        }

        SECTION("Empty placeholder errors")
        {
            auto fmt = Value::create("${}"s);
            auto repl = Value::create(Map{});

            CHECK_THROWS_WITH(mformat->call({fmt, repl}),
                              "Invalid format placeholder: ${}");
        }

        SECTION("Invalid placeholder identifier errors")
        {
            SECTION("Starts with a digit")
            {
                auto fmt = Value::create("${1abc}"s);
                auto repl = Value::create(Map{
                    {Value::create("1abc"s), Value::create("x"s)},
                });

                CHECK_THROWS_MATCHES(
                    mformat->call({fmt, repl}), Frost_User_Error,
                    MessageMatches(
                        ContainsSubstring("Invalid format placeholder")
                        && ContainsSubstring("1abc")));
            }

            SECTION("Contains non-identifier characters")
            {
                auto fmt = Value::create("${a-b}"s);
                auto repl = Value::create(Map{
                    {Value::create("a-b"s), Value::create("x"s)},
                });

                CHECK_THROWS_MATCHES(
                    mformat->call({fmt, repl}), Frost_User_Error,
                    MessageMatches(
                        ContainsSubstring("Invalid format placeholder")
                        && ContainsSubstring("a-b")));
            }
        }

        SECTION("Pathological format strings do not throw unexpectedly")
        {
            auto repl = Value::create(Map{
                {Value::create("a"s), Value::create("x"s)},
                {Value::create("_"s), Value::create("y"s)},
            });

            SECTION("Trailing dollar")
            {
                auto fmt = Value::create("$"s);
                auto res = mformat->call({fmt, repl});
                REQUIRE(res->is<String>());
                CHECK(res->get<String>().value() == "$");
            }

            SECTION("Many dollars without braces")
            {
                auto fmt = Value::create("$$$$$$$"s);
                auto res = mformat->call({fmt, repl});
                REQUIRE(res->is<String>());
                CHECK(res->get<String>().value() == "$$$$$$$");
            }

            SECTION("Dollar followed by brace-like text but no placeholder")
            {
                auto fmt = Value::create("$ { } $ { }"s);
                auto res = mformat->call({fmt, repl});
                REQUIRE(res->is<String>());
                CHECK(res->get<String>().value() == "$ { } $ { }");
            }

            SECTION("Lone closing brace is literal")
            {
                auto fmt = Value::create("}"s);
                auto res = mformat->call({fmt, repl});
                REQUIRE(res->is<String>());
                CHECK(res->get<String>().value() == "}");
            }

            SECTION("Many closing braces are literal")
            {
                auto fmt = Value::create("}}}"s);
                auto res = mformat->call({fmt, repl});
                REQUIRE(res->is<String>());
                CHECK(res->get<String>().value() == "}}}");
            }

            SECTION("Extra closing brace after placeholder is literal")
            {
                auto fmt = Value::create("${a}}"s);
                auto res = mformat->call({fmt, repl});
                REQUIRE(res->is<String>());
                CHECK(res->get<String>().value() == "x}");
            }

            SECTION("Placeholder followed by opening brace is literal")
            {
                auto fmt = Value::create("${a}{"s);
                auto res = mformat->call({fmt, repl});
                REQUIRE(res->is<String>());
                CHECK(res->get<String>().value() == "x{");
            }

            SECTION("Dollar directly before placeholder keeps literal dollar")
            {
                auto fmt = Value::create("$ ${a}"s);
                auto res = mformat->call({fmt, repl});
                REQUIRE(res->is<String>());
                CHECK(res->get<String>().value() == "$ x");
            }

            SECTION("Stray closing brace before placeholder is literal")
            {
                auto fmt = Value::create("}${a}"s);
                auto res = mformat->call({fmt, repl});
                REQUIRE(res->is<String>());
                CHECK(res->get<String>().value() == "}x");
            }

            SECTION("Nested placeholder text triggers error")
            {
                auto fmt = Value::create("${${a}}"s);
                CHECK_THROWS_MATCHES(
                    mformat->call({fmt, repl}), std::exception,
                    MessageMatches(
                        ContainsSubstring("Invalid format placeholder")
                        && ContainsSubstring("${a")));
            }

            SECTION("Nested placeholder with extra text triggers error")
            {
                auto fmt = Value::create("${x${a}y}"s);
                CHECK_THROWS_MATCHES(
                    mformat->call({fmt, repl}), std::exception,
                    MessageMatches(
                        ContainsSubstring("Invalid format placeholder")
                        && ContainsSubstring("x${a")));
            }

            SECTION("Nested placeholder with suffix triggers error")
            {
                auto fmt = Value::create("${a${a}}"s);
                CHECK_THROWS_MATCHES(
                    mformat->call({fmt, repl}), std::exception,
                    MessageMatches(
                        ContainsSubstring("Invalid format placeholder")
                        && ContainsSubstring("a${a")));
            }

            SECTION("Multiple placeholders with trailing unterminated error")
            {
                auto fmt = Value::create("${a}${a}${"s);
                CHECK_THROWS_MATCHES(mformat->call({fmt, repl}), std::exception,
                                     MessageMatches(ContainsSubstring(
                                         "Unterminated format placeholder")));
            }

            SECTION("Multiple placeholder markers with one missing brace")
            {
                auto fmt = Value::create("x${a}y${"s);
                CHECK_THROWS_MATCHES(mformat->call({fmt, repl}), std::exception,
                                     MessageMatches(ContainsSubstring(
                                         "Unterminated format placeholder")));
            }

            SECTION("Just placeholder opener")
            {
                auto fmt = Value::create("${"s);
                CHECK_THROWS_MATCHES(mformat->call({fmt, repl}), std::exception,
                                     MessageMatches(ContainsSubstring(
                                         "Unterminated format placeholder")));
            }

            SECTION("Whitespace inside placeholder errors")
            {
                auto fmt = Value::create("${ a}"s);
                CHECK_THROWS_MATCHES(
                    mformat->call({fmt, repl}), std::exception,
                    MessageMatches(
                        ContainsSubstring("Invalid format placeholder")
                        && ContainsSubstring(" a")));

                auto fmt_suffix = Value::create("${a }"s);
                CHECK_THROWS_MATCHES(
                    mformat->call({fmt_suffix, repl}), std::exception,
                    MessageMatches(
                        ContainsSubstring("Invalid format placeholder")
                        && ContainsSubstring("a ")));

                auto fmt_tab = Value::create("${a\t}"s);
                CHECK_THROWS_MATCHES(
                    mformat->call({fmt_tab, repl}), std::exception,
                    MessageMatches(
                        ContainsSubstring("Invalid format placeholder")
                        && ContainsSubstring("a\t")));
            }

            SECTION("Non-ASCII placeholder identifiers error")
            {
                auto fmt = Value::create("${naïve}"s);
                CHECK_THROWS_MATCHES(
                    mformat->call({fmt, repl}), std::exception,
                    MessageMatches(
                        ContainsSubstring("Invalid format placeholder")
                        && ContainsSubstring("naïve")));

                auto fmt_emoji = Value::create("${😀}"s);
                CHECK_THROWS_MATCHES(
                    mformat->call({fmt_emoji, repl}), std::exception,
                    MessageMatches(
                        ContainsSubstring("Invalid format placeholder")
                        && ContainsSubstring("😀")));
            }

            SECTION("Very long identifier is accepted")
            {
                std::string key(1024, 'a');
                auto fmt = Value::create("${" + key + "}"s);
                auto repl_long = Value::create(Map{
                    {Value::create(auto{key}), Value::create("ok"s)},
                });

                auto res = mformat->call({fmt, repl_long});
                REQUIRE(res->is<String>());
                CHECK(res->get<String>().value() == "ok");
            }
        }
    }
}
