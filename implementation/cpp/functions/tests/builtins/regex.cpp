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
                REQUIRE(v->is<Function>());
                return v->get<Function>().value();
            }
        }
        FAIL("Missing re function");
        return Function{};
    };

    const std::vector<std::string> names{"matches", "contains"};

    SECTION("Injected")
    {
        CHECK(re_val->is<Map>());
        CHECK(re_map.size() == 2);

        for (const auto& name : names)
        {
            auto fn = get_re_fn(name);
            REQUIRE(fn);
        }
    }

    SECTION("Arity")
    {
        for (const auto& name : names)
        {
            DYNAMIC_SECTION(name << " arity")
            {
                auto fn = get_re_fn(name);

                CHECK_THROWS_WITH(fn->call({}),
                                  ContainsSubstring("insufficient arguments"));
                CHECK_THROWS_WITH(fn->call({}),
                                  ContainsSubstring("Called with 0"));
                CHECK_THROWS_WITH(fn->call({}),
                                  ContainsSubstring("requires at least 2"));

                CHECK_THROWS_WITH(fn->call({Value::create("a"s),
                                            Value::create("b"s),
                                            Value::create("c"s)}),
                                  ContainsSubstring("too many arguments"));
                CHECK_THROWS_WITH(fn->call({Value::create("a"s),
                                            Value::create("b"s),
                                            Value::create("c"s)}),
                                  ContainsSubstring("Called with 3"));
                CHECK_THROWS_WITH(fn->call({Value::create("a"s),
                                            Value::create("b"s),
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

        for (const auto& name : names)
        {
            DYNAMIC_SECTION(name << " type errors")
            {
                auto fn = get_re_fn(name);
                CHECK_THROWS_WITH(fn->call({bad_first, good}),
                                  ContainsSubstring(std::string{"Function "}
                                                    + name));
                CHECK_THROWS_WITH(fn->call({bad_first, good}),
                                  ContainsSubstring("String"));
                CHECK_THROWS_WITH(fn->call({bad_first, good}),
                                  EndsWith(std::string{bad_first->type_name()}));

                CHECK_THROWS_WITH(fn->call({good, bad_second}),
                                  ContainsSubstring(std::string{"Function "}
                                                    + name));
                CHECK_THROWS_WITH(fn->call({good, bad_second}),
                                  ContainsSubstring("String"));
                CHECK_THROWS_WITH(fn->call({good, bad_second}),
                                  EndsWith(std::string{bad_second->type_name()}));
            }
        }
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
        CHECK(get_re_fn("matches")->call({empty_target, empty})
                  ->get<Bool>()
                  .value());
    }

    SECTION("Matches vs contains")
    {
        auto target = Value::create("foofoobar"s);
        auto pattern = Value::create("(foo)+"s);

        CHECK_FALSE(
            get_re_fn("matches")->call({target, pattern})->get<Bool>().value());
        CHECK(
            get_re_fn("contains")->call({target, pattern})->get<Bool>().value());

        auto full = Value::create("foofoo"s);
        CHECK(get_re_fn("matches")->call({full, pattern})->get<Bool>().value());
    }

    SECTION("Case sensitivity")
    {
        auto target = Value::create("foo"s);
        auto pattern = Value::create("Foo"s);

        CHECK_FALSE(
            get_re_fn("matches")->call({target, pattern})->get<Bool>().value());
        CHECK_FALSE(
            get_re_fn("contains")->call({target, pattern})->get<Bool>().value());
    }

    SECTION("UTF-8 literals")
    {
        auto target = Value::create("hi 😊 frost 😊"s);
        auto emoji = Value::create("😊"s);
        auto full = Value::create("hi 😊 frost 😊"s);

        CHECK(get_re_fn("contains")->call({target, emoji})->get<Bool>().value());
        CHECK(get_re_fn("matches")->call({target, full})->get<Bool>().value());
    }
}
