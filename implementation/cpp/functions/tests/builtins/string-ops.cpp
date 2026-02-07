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

TEST_CASE("Builtin contains/starts_with/ends_with")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    Symbol_Table table;
    inject_builtins(table);

    struct Builtin_Info
    {
        const char* name;
    };

    const std::vector<Builtin_Info> builtins{
        {.name = "contains"},
        {.name = "starts_with"},
        {.name = "ends_with"},
    };

    auto get_fn = [&](std::string_view name) {
        auto val = table.lookup(std::string{name});
        REQUIRE(val->is<Function>());
        return val->get<Function>().value();
    };

    SECTION("Injected")
    {
        for (const auto& info : builtins)
        {
            auto val = table.lookup(info.name);
            REQUIRE(val->is<Function>());
        }
    }

    SECTION("Arity")
    {
        for (const auto& info : builtins)
        {
            DYNAMIC_SECTION(info.name << " arity")
            {
                auto fn = get_fn(info.name);
                CHECK_THROWS_WITH(fn->call({}),
                                  ContainsSubstring("insufficient arguments"));
                CHECK_THROWS_WITH(fn->call({}),
                                  ContainsSubstring("Called with 0"));
                CHECK_THROWS_WITH(fn->call({}),
                                  ContainsSubstring("requires at least 2"));

                CHECK_THROWS_WITH(
                    fn->call({Value::create("a"s), Value::create("b"s),
                              Value::create("c"s)}),
                    ContainsSubstring("too many arguments"));
                CHECK_THROWS_WITH(
                    fn->call({Value::create("a"s), Value::create("b"s),
                              Value::create("c"s)}),
                    ContainsSubstring("Called with 3"));
                CHECK_THROWS_WITH(
                    fn->call({Value::create("a"s), Value::create("b"s),
                              Value::create("c"s)}),
                    ContainsSubstring("no more than 2"));
            }
        }
    }

    SECTION("Type errors")
    {
        auto bad_first = Value::create(1_f);
        auto bad_second = Value::create(true);
        auto good = Value::create("a"s);

        for (const auto& info : builtins)
        {
            DYNAMIC_SECTION(info.name << " type errors")
            {
                auto fn = get_fn(info.name);
                CHECK_THROWS_WITH(
                    fn->call({bad_first, good}),
                    ContainsSubstring(std::string{"Function "} + info.name));
                CHECK_THROWS_WITH(fn->call({bad_first, good}),
                                  ContainsSubstring("String"));
                CHECK_THROWS_WITH(
                    fn->call({bad_first, good}),
                    EndsWith(std::string{bad_first->type_name()}));

                CHECK_THROWS_WITH(
                    fn->call({good, bad_second}),
                    ContainsSubstring(std::string{"Function "} + info.name));
                CHECK_THROWS_WITH(fn->call({good, bad_second}),
                                  ContainsSubstring("String"));
                CHECK_THROWS_WITH(
                    fn->call({good, bad_second}),
                    EndsWith(std::string{bad_second->type_name()}));
            }
        }
    }

    SECTION("Empty substring returns true")
    {
        auto target = Value::create("abc"s);
        auto empty = Value::create(""s);

        CHECK(get_fn("contains")->call({target, empty})->get<Bool>().value());
        CHECK(
            get_fn("starts_with")->call({target, empty})->get<Bool>().value());
        CHECK(get_fn("ends_with")->call({target, empty})->get<Bool>().value());
    }

    SECTION("Basic behavior")
    {
        auto target = Value::create("hello world"s);
        auto sub = Value::create("lo wo"s);
        auto prefix = Value::create("hello"s);
        auto suffix = Value::create("world"s);
        auto missing = Value::create("nope"s);

        CHECK(get_fn("contains")->call({target, sub})->get<Bool>().value());
        CHECK_FALSE(
            get_fn("contains")->call({target, missing})->get<Bool>().value());

        CHECK(
            get_fn("starts_with")->call({target, prefix})->get<Bool>().value());
        CHECK_FALSE(
            get_fn("starts_with")->call({target, sub})->get<Bool>().value());

        CHECK(get_fn("ends_with")->call({target, suffix})->get<Bool>().value());
        CHECK_FALSE(
            get_fn("ends_with")->call({target, sub})->get<Bool>().value());
    }

    SECTION("UTF-8 behavior")
    {
        auto target = Value::create("hi 😊 frost 😊"s);
        auto emoji = Value::create("😊"s);
        auto prefix = Value::create("hi 😊"s);
        auto suffix = Value::create("frost 😊"s);

        CHECK(get_fn("contains")->call({target, emoji})->get<Bool>().value());
        CHECK(
            get_fn("starts_with")->call({target, prefix})->get<Bool>().value());
        CHECK(get_fn("ends_with")->call({target, suffix})->get<Bool>().value());
    }
}

TEST_CASE("Builtin to_upper/to_lower")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    Symbol_Table table;
    inject_builtins(table);

    struct Builtin_Info
    {
        const char* name;
    };

    const std::vector<Builtin_Info> builtins{
        {.name = "to_upper"},
        {.name = "to_lower"},
    };

    auto get_fn = [&](std::string_view name) {
        auto val = table.lookup(std::string{name});
        REQUIRE(val->is<Function>());
        return val->get<Function>().value();
    };

    SECTION("Injected")
    {
        for (const auto& info : builtins)
        {
            auto val = table.lookup(info.name);
            REQUIRE(val->is<Function>());
        }
    }

    SECTION("Arity")
    {
        for (const auto& info : builtins)
        {
            DYNAMIC_SECTION(info.name << " arity")
            {
                auto fn = get_fn(info.name);
                CHECK_THROWS_WITH(fn->call({}),
                                  ContainsSubstring("insufficient arguments"));
                CHECK_THROWS_WITH(fn->call({}),
                                  ContainsSubstring("Called with 0"));
                CHECK_THROWS_WITH(fn->call({}),
                                  ContainsSubstring("requires at least 1"));

                CHECK_THROWS_WITH(
                    fn->call({Value::create("a"s), Value::create("extra"s)}),
                    ContainsSubstring("too many arguments"));
                CHECK_THROWS_WITH(
                    fn->call({Value::create("a"s), Value::create("extra"s)}),
                    ContainsSubstring("Called with 2"));
                CHECK_THROWS_WITH(
                    fn->call({Value::create("a"s), Value::create("extra"s)}),
                    ContainsSubstring("no more than 1"));
            }
        }
    }

    SECTION("Type errors")
    {
        auto bad = Value::create(1_f);
        auto good = Value::create("a"s);

        for (const auto& info : builtins)
        {
            DYNAMIC_SECTION(info.name << " type errors")
            {
                auto fn = get_fn(info.name);
                CHECK_THROWS_WITH(
                    fn->call({bad}),
                    ContainsSubstring(std::string{"Function "} + info.name));
                CHECK_THROWS_WITH(fn->call({bad}),
                                  ContainsSubstring("String"));
                CHECK_THROWS_WITH(fn->call({bad}),
                                  EndsWith(std::string{bad->type_name()}));

                // Ensure good input still works after a type error.
                CHECK_NOTHROW(fn->call({good}));
            }
        }
    }

    SECTION("Basic behavior")
    {
        auto input = Value::create("AbC123_!zZ"s);

        auto upper =
            get_fn("to_upper")->call({input})->get<String>().value();
        auto lower =
            get_fn("to_lower")->call({input})->get<String>().value();

        CHECK(upper == "ABC123_!ZZ");
        CHECK(lower == "abc123_!zz");
    }

    SECTION("Empty string")
    {
        auto empty = Value::create(""s);
        CHECK(get_fn("to_upper")->call({empty})->get<String>().value().empty());
        CHECK(get_fn("to_lower")->call({empty})->get<String>().value().empty());
    }
}
