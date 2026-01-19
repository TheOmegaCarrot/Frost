// AI-generated test by Codex (GPT-5).
// Signed: Codex (GPT-5).
#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <frost/builtin.hpp>

using namespace frst;

using namespace std::literals;

using namespace Catch::Matchers;

TEST_CASE("Builtin regex")
{
    Symbol_Table table;
    inject_builtins(table);

    auto re_val = table.lookup("re");
    REQUIRE(re_val->is<Map>());
    const auto& re_map = re_val->raw_get<Map>();

    auto get_re_fn = [&](std::string_view name) {
        auto key = Value::create(std::string{name});
        for (const auto& [k, v] : re_map)
        {
            if (Value::equal(k, key)->get<Bool>().value())
            {
                REQUIRE(v);
                REQUIRE(v->is<Function>());
                return v->get<Function>().value();
            }
        }
        FAIL("Missing re function");
        return Function{};
    };

    const std::vector<std::string> names{"matches", "contains"};
    const std::string replace_name{"replace"};

    SECTION("Injected")
    {
        CHECK(re_val->is<Map>());
        CHECK(re_map.size() == 3);

        for (const auto& name : names)
        {
            auto fn = get_re_fn(name);
            REQUIRE(fn);
        }
        REQUIRE(get_re_fn(replace_name));
    }

    SECTION("Arity")
    {
        for (const auto& name : names)
        {
            DYNAMIC_SECTION(name << " arity")
            {
                auto fn = get_re_fn(name);

                const auto too_few_1 =
                    ContainsSubstring("insufficient arguments");
                const auto too_few_2 = ContainsSubstring("Called with 0");
                const auto too_few_3 = ContainsSubstring("requires at least 2");
                CHECK_THROWS_WITH(fn->call({}),
                                  too_few_1 && too_few_2 && too_few_3);

                const auto too_many_1 = ContainsSubstring("too many arguments");
                const auto too_many_2 = ContainsSubstring("Called with 3");
                const auto too_many_3 = ContainsSubstring("no more than 2");
                CHECK_THROWS_WITH(
                    fn->call({Value::create("a"s), Value::create("b"s),
                              Value::create("c"s)}),
                    too_many_1 && too_many_2 && too_many_3);
            }
        }
    }

    SECTION("Replace arity")
    {
        auto fn = get_re_fn(replace_name);

        const auto too_few_1 = ContainsSubstring("insufficient arguments");
        const auto too_few_2 = ContainsSubstring("Called with 2");
        const auto too_few_3 = ContainsSubstring("requires at least 3");
        CHECK_THROWS_WITH(
            fn->call({Value::create("a"s), Value::create("b"s)}),
            too_few_1 && too_few_2 && too_few_3);

        const auto too_many_1 = ContainsSubstring("too many arguments");
        const auto too_many_2 = ContainsSubstring("Called with 4");
        const auto too_many_3 = ContainsSubstring("no more than 3");
        CHECK_THROWS_WITH(fn->call({Value::create("a"s), Value::create("b"s),
                                    Value::create("c"s),
                                    Value::create("d"s)}),
                          too_many_1 && too_many_2 && too_many_3);
    }

    SECTION("Type errors")
    {
        auto bad_first = Value::create(1_f);
        auto bad_second = Value::create(true);
        auto good = Value::create("a"s);

        for (const auto& name : names)
        {
            DYNAMIC_SECTION(name << " type errors")
            {
                auto fn = get_re_fn(name);
                const auto first_fn =
                    ContainsSubstring(std::string{"Function re."} + name);
                const auto first_string = ContainsSubstring("String");
                const auto first_type =
                    EndsWith(std::string{bad_first->type_name()});
                CHECK_THROWS_WITH(fn->call({bad_first, good}),
                                  first_fn && first_string && first_type);

                const auto second_fn =
                    ContainsSubstring(std::string{"Function re."} + name);
                const auto second_string = ContainsSubstring("String");
                const auto second_type =
                    EndsWith(std::string{bad_second->type_name()});
                CHECK_THROWS_WITH(fn->call({good, bad_second}),
                                  second_fn && second_string && second_type);
            }
        }
    }

    SECTION("Replace type errors")
    {
        auto bad_first = Value::create(1_f);
        auto bad_second = Value::create(true);
        auto bad_third = Value::create(Null{});
        auto good = Value::create("a"s);
        auto fn = get_re_fn(replace_name);

        const auto fn_name =
            ContainsSubstring(std::string{"Function re."} + replace_name);
        const auto expected = ContainsSubstring("String");

        CHECK_THROWS_WITH(fn->call({bad_first, good, good}),
                          fn_name && expected
                              && EndsWith(std::string{bad_first->type_name()}));
        CHECK_THROWS_WITH(
            fn->call({good, bad_second, good}),
            fn_name && expected
                && EndsWith(std::string{bad_second->type_name()}));
        CHECK_THROWS_WITH(
            fn->call({good, good, bad_third}),
            fn_name && expected
                && EndsWith(std::string{bad_third->type_name()}));
    }

    SECTION("Regex syntax errors")
    {
        auto target = Value::create("abc"s);
        auto bad_re = Value::create("("s);

        for (const auto& name : names)
        {
            DYNAMIC_SECTION(name << " regex error")
            {
                auto fn = get_re_fn(name);
                CHECK_THROWS_MATCHES(
                    fn->call({target, bad_re}), Frost_User_Error,
                    MessageMatches(StartsWith("Regex error: ")));
            }
        }

        {
            auto fn = get_re_fn(replace_name);
            CHECK_THROWS_MATCHES(
                fn->call({target, bad_re, Value::create("x"s)}),
                Frost_User_Error, MessageMatches(StartsWith("Regex error: ")));
        }
    }

    SECTION("Empty regex matches anything")
    {
        auto target = Value::create("abc"s);
        auto empty = Value::create(""s);

        CHECK_FALSE(
            get_re_fn("matches")->call({target, empty})->get<Bool>().value());
        CHECK(
            get_re_fn("contains")->call({target, empty})->get<Bool>().value());

        auto empty_target = Value::create(""s);
        CHECK(get_re_fn("matches")
                  ->call({empty_target, empty})
                  ->get<Bool>()
                  .value());
    }

    SECTION("Replace success")
    {
        auto target = Value::create("a1b2"s);
        auto pattern = Value::create("[0-9]"s);
        auto replacement = Value::create("X"s);

        CHECK(get_re_fn("replace")
                  ->call({target, pattern, replacement})
                  ->raw_get<String>()
              == "aXbX");

        auto capture_target = Value::create("foo"s);
        auto capture_pattern = Value::create("(foo)"s);
        auto capture_repl = Value::create("<$1>"s);

        CHECK(get_re_fn("replace")
                  ->call({capture_target, capture_pattern, capture_repl})
                  ->raw_get<String>()
              == "<foo>");
    }

    SECTION("Matches vs contains")
    {
        auto target = Value::create("foofoobar"s);
        auto pattern = Value::create("(foo)+"s);

        CHECK_FALSE(
            get_re_fn("matches")->call({target, pattern})->get<Bool>().value());
        CHECK(get_re_fn("contains")
                  ->call({target, pattern})
                  ->get<Bool>()
                  .value());

        auto full = Value::create("foofoo"s);
        CHECK(get_re_fn("matches")->call({full, pattern})->get<Bool>().value());
    }

    SECTION("Case sensitivity")
    {
        auto target = Value::create("foo"s);
        auto pattern = Value::create("Foo"s);

        CHECK_FALSE(
            get_re_fn("matches")->call({target, pattern})->get<Bool>().value());
        CHECK_FALSE(get_re_fn("contains")
                        ->call({target, pattern})
                        ->get<Bool>()
                        .value());
    }

    SECTION("UTF-8 literals")
    {
        auto target = Value::create("hi 😊 frost 😊"s);
        auto emoji = Value::create("😊"s);
        auto full = Value::create("hi 😊 frost 😊"s);

        CHECK(
            get_re_fn("contains")->call({target, emoji})->get<Bool>().value());
        CHECK(get_re_fn("matches")->call({target, full})->get<Bool>().value());
    }
}
