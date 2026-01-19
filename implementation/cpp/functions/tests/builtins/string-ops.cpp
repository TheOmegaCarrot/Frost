#include <catch2/catch_all.hpp>

#include <frost/mock/mock-callable.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <frost/builtin.hpp>

using namespace frst;

using namespace std::literals;

using namespace Catch::Matchers;

TEST_CASE("Builtin split")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    Symbol_Table table;
    inject_builtins(table);
    auto split_val = table.lookup("split");
    REQUIRE(split_val->is<Function>());
    auto split = split_val->get<Function>().value();

    SECTION("Injected")
    {
        CHECK(split_val->is<Function>());
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(split->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(split->call({}), ContainsSubstring("Called with 0"));
        CHECK_THROWS_WITH(split->call({}),
                          ContainsSubstring("requires at least 2"));

        CHECK_THROWS_WITH(split->call({Value::create("a"s), Value::create(","s),
                                       Value::create("extra"s)}),
                          ContainsSubstring("too many arguments"));
        CHECK_THROWS_WITH(split->call({Value::create("a"s), Value::create(","s),
                                       Value::create("extra"s)}),
                          ContainsSubstring("Called with 3"));
        CHECK_THROWS_WITH(split->call({Value::create("a"s), Value::create(","s),
                                       Value::create("extra"s)}),
                          ContainsSubstring("no more than 2"));
    }

    SECTION("Type errors")
    {
        auto bad_first = Value::create(1_f);
        auto bad_second = Value::create(true);
        auto good = Value::create("a"s);

        CHECK_THROWS_WITH(split->call({bad_first, good}),
                          ContainsSubstring("Function split"));
        CHECK_THROWS_WITH(split->call({bad_first, good}),
                          ContainsSubstring("String"));
        CHECK_THROWS_WITH(split->call({bad_first, good}),
                          EndsWith(std::string{bad_first->type_name()}));

        CHECK_THROWS_WITH(split->call({good, bad_second}),
                          ContainsSubstring("Function split"));
        CHECK_THROWS_WITH(split->call({good, bad_second}),
                          ContainsSubstring("String"));
        CHECK_THROWS_WITH(split->call({good, bad_second}),
                          EndsWith(std::string{bad_second->type_name()}));
    }

    SECTION("Delimiter length errors")
    {
        auto target = Value::create("a,b"s);

        CHECK_THROWS_WITH(
            split->call({target, Value::create(""s)}),
            ContainsSubstring(
                "Function split expected second argument to be length 1"));
        CHECK_THROWS_WITH(split->call({target, Value::create(""s)}),
                          ContainsSubstring("length 0"));

        CHECK_THROWS_WITH(
            split->call({target, Value::create("ab"s)}),
            ContainsSubstring(
                "Function split expected second argument to be length 1"));
        CHECK_THROWS_WITH(split->call({target, Value::create("ab"s)}),
                          ContainsSubstring("length 2"));
    }

    SECTION("Empty string returns empty array")
    {
        auto res = split->call({Value::create(""s), Value::create(","s)});
        REQUIRE(res->is<Array>());
        CHECK(res->get<Array>().value().empty());
    }

    SECTION("No delimiter yields one element")
    {
        auto res = split->call({Value::create("abc"s), Value::create(","s)});
        REQUIRE(res->is<Array>());
        auto arr = res->get<Array>().value();
        REQUIRE(arr.size() == 1);
        CHECK(arr.at(0)->get<String>().value() == "abc");
    }

    SECTION("Consecutive delimiters yield empty fields")
    {
        auto res = split->call({Value::create("a,,b"s), Value::create(","s)});
        REQUIRE(res->is<Array>());
        auto arr = res->get<Array>().value();
        REQUIRE(arr.size() == 3);
        CHECK(arr.at(0)->get<String>().value() == "a");
        CHECK(arr.at(1)->get<String>().value() == "");
        CHECK(arr.at(2)->get<String>().value() == "b");
    }

    SECTION("Leading and trailing delimiters yield empty fields")
    {
        auto res = split->call({Value::create(",a,"s), Value::create(","s)});
        REQUIRE(res->is<Array>());
        auto arr = res->get<Array>().value();
        REQUIRE(arr.size() == 3);
        CHECK(arr.at(0)->get<String>().value() == "");
        CHECK(arr.at(1)->get<String>().value() == "a");
        CHECK(arr.at(2)->get<String>().value() == "");
    }

    SECTION("Basic split")
    {
        auto res = split->call({Value::create("a,b,c"s), Value::create(","s)});
        REQUIRE(res->is<Array>());
        auto arr = res->get<Array>().value();
        REQUIRE(arr.size() == 3);
        CHECK(arr.at(0)->get<String>().value() == "a");
        CHECK(arr.at(1)->get<String>().value() == "b");
        CHECK(arr.at(2)->get<String>().value() == "c");
    }
}
