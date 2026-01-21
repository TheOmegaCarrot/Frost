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
    const std::string scan_matches_name{"scan_matches"};

    SECTION("Injected")
    {
        CHECK(re_val->is<Map>());
        CHECK(re_map.size() == 4);

        for (const auto& name : names)
        {
            auto fn = get_re_fn(name);
            REQUIRE(fn);
        }
        REQUIRE(get_re_fn(replace_name));
        REQUIRE(get_re_fn(scan_matches_name));
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
        CHECK_THROWS_WITH(fn->call({Value::create("a"s), Value::create("b"s)}),
                          too_few_1 && too_few_2 && too_few_3);

        const auto too_many_1 = ContainsSubstring("too many arguments");
        const auto too_many_2 = ContainsSubstring("Called with 4");
        const auto too_many_3 = ContainsSubstring("no more than 3");
        CHECK_THROWS_WITH(fn->call({Value::create("a"s), Value::create("b"s),
                                    Value::create("c"s), Value::create("d"s)}),
                          too_many_1 && too_many_2 && too_many_3);
    }

    SECTION("Scan matches arity")
    {
        auto fn = get_re_fn(scan_matches_name);

        const auto too_few_1 = ContainsSubstring("insufficient arguments");
        const auto too_few_2 = ContainsSubstring("Called with 1");
        const auto too_few_3 = ContainsSubstring("requires at least 2");
        CHECK_THROWS_WITH(fn->call({Value::create("a"s)}),
                          too_few_1 && too_few_2 && too_few_3);

        const auto too_many_1 = ContainsSubstring("too many arguments");
        const auto too_many_2 = ContainsSubstring("Called with 3");
        const auto too_many_3 = ContainsSubstring("no more than 2");
        CHECK_THROWS_WITH(
            fn->call({Value::create("a"s), Value::create("b"s),
                      Value::create("c"s)}),
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
                          fn_name
                              && expected
                              && EndsWith(std::string{bad_first->type_name()}));
        CHECK_THROWS_WITH(
            fn->call({good, bad_second, good}),
            fn_name
                && expected
                && EndsWith(std::string{bad_second->type_name()}));
        CHECK_THROWS_WITH(fn->call({good, good, bad_third}),
                          fn_name
                              && expected
                              && EndsWith(std::string{bad_third->type_name()}));
    }

    SECTION("Scan matches type errors")
    {
        auto bad_first = Value::create(1_f);
        auto bad_second = Value::create(true);
        auto good = Value::create("a"s);
        auto fn = get_re_fn(scan_matches_name);

        const auto fn_name = ContainsSubstring(std::string{"Function re."}
                                               + scan_matches_name);
        const auto expected = ContainsSubstring("String");

        CHECK_THROWS_WITH(fn->call({bad_first, good}),
                          fn_name
                              && expected
                              && EndsWith(std::string{bad_first->type_name()}));
        CHECK_THROWS_WITH(
            fn->call({good, bad_second}),
            fn_name
                && expected
                && EndsWith(std::string{bad_second->type_name()}));
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

        {
            auto fn = get_re_fn(scan_matches_name);
            CHECK_THROWS_MATCHES(
                fn->call({target, bad_re}), Frost_User_Error,
                MessageMatches(StartsWith("Regex error: ")));
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

    SECTION("Scan matches no matches")
    {
        auto fn = get_re_fn(scan_matches_name);
        auto result =
            fn->call({Value::create("abc"s), Value::create("z+"s)});
        REQUIRE(result->is<Map>());
        const auto& result_map = result->raw_get<Map>();

        auto get_field = [&](const Map& map, std::string_view key) {
            auto it = map.find(Value::create(std::string{key}));
            REQUIRE(it != map.end());
            return it->second;
        };

        CHECK_FALSE(get_field(result_map, "found")->get<Bool>().value());
        CHECK(get_field(result_map, "count")->raw_get<Int>() == 0);

        auto matches = get_field(result_map, "matches");
        REQUIRE(matches->is<Array>());
        CHECK(matches->raw_get<Array>().empty());
    }

    SECTION("Scan matches with groups")
    {
        auto fn = get_re_fn(scan_matches_name);
        auto result = fn->call(
            {Value::create("foo=bar beep=boop"s),
             Value::create(R"((\w+)=(\w+))"s)});
        REQUIRE(result->is<Map>());
        const auto& result_map = result->raw_get<Map>();

        auto get_field = [&](const Map& map, std::string_view key) {
            auto it = map.find(Value::create(std::string{key}));
            REQUIRE(it != map.end());
            return it->second;
        };

        CHECK(get_field(result_map, "found")->get<Bool>().value());
        CHECK(get_field(result_map, "count")->raw_get<Int>() == 2);

        auto matches_val = get_field(result_map, "matches");
        REQUIRE(matches_val->is<Array>());
        const auto& matches = matches_val->raw_get<Array>();
        REQUIRE(matches.size() == 2);

        auto match0 = matches.at(0);
        REQUIRE(match0->is<Map>());
        const auto& match0_map = match0->raw_get<Map>();

        auto groups_val = get_field(match0_map, "groups");
        REQUIRE(groups_val->is<Array>());
        const auto& groups = groups_val->raw_get<Array>();
        REQUIRE(groups.size() == 3);

        auto full_val = get_field(match0_map, "full");
        auto group0 = groups.at(0);
        REQUIRE(group0->is<Map>());
        const auto& group0_map = group0->raw_get<Map>();
        auto group0_value = get_field(group0_map, "value");
        CHECK(full_val == group0_value);

        CHECK(get_field(group0_map, "index")->raw_get<Int>() == 0);
        CHECK(get_field(group0_map, "matched")->get<Bool>().value());
        CHECK(get_field(group0_map, "value")->raw_get<String>() == "foo=bar");

        auto group1 = groups.at(1);
        REQUIRE(group1->is<Map>());
        const auto& group1_map = group1->raw_get<Map>();
        CHECK(get_field(group1_map, "index")->raw_get<Int>() == 1);
        CHECK(get_field(group1_map, "matched")->get<Bool>().value());
        CHECK(get_field(group1_map, "value")->raw_get<String>() == "foo");

        auto group2 = groups.at(2);
        REQUIRE(group2->is<Map>());
        const auto& group2_map = group2->raw_get<Map>();
        CHECK(get_field(group2_map, "index")->raw_get<Int>() == 2);
        CHECK(get_field(group2_map, "matched")->get<Bool>().value());
        CHECK(get_field(group2_map, "value")->raw_get<String>() == "bar");

        CHECK(match0_map.find(Value::create("named"s)) == match0_map.end());

        auto match1 = matches.at(1);
        REQUIRE(match1->is<Map>());
        const auto& match1_map = match1->raw_get<Map>();
        CHECK(match1_map.find(Value::create("named"s)) == match1_map.end());
    }

    SECTION("Scan matches named groups")
    {
        auto fn = get_re_fn(scan_matches_name);
        auto result = fn->call(
            {Value::create("foo=bar beep=boop"s),
             Value::create(R"((?<k>\w+)=(?'v'\w+))"s)});
        REQUIRE(result->is<Map>());
        const auto& result_map = result->raw_get<Map>();

        auto get_field = [&](const Map& map, std::string_view key) {
            auto it = map.find(Value::create(std::string{key}));
            REQUIRE(it != map.end());
            return it->second;
        };

        auto matches_val = get_field(result_map, "matches");
        REQUIRE(matches_val->is<Array>());
        const auto& matches = matches_val->raw_get<Array>();
        REQUIRE(matches.size() == 2);

        auto match0 = matches.at(0);
        REQUIRE(match0->is<Map>());
        const auto& match0_map = match0->raw_get<Map>();
        auto named_val = get_field(match0_map, "named");
        REQUIRE(named_val->is<Map>());
        const auto& named_map = named_val->raw_get<Map>();

        auto named_k = get_field(named_map, "k");
        auto named_v = get_field(named_map, "v");
        REQUIRE(named_k->is<Map>());
        REQUIRE(named_v->is<Map>());

        const auto& k_map = named_k->raw_get<Map>();
        const auto& v_map = named_v->raw_get<Map>();
        CHECK(get_field(k_map, "matched")->get<Bool>().value());
        CHECK(get_field(k_map, "value")->raw_get<String>() == "foo");
        CHECK(get_field(v_map, "matched")->get<Bool>().value());
        CHECK(get_field(v_map, "value")->raw_get<String>() == "bar");
    }

    SECTION("Scan matches named groups with optional capture")
    {
        auto fn = get_re_fn(scan_matches_name);
        auto result =
            fn->call({Value::create("foo bar1"s),
                      Value::create(R"((?<k>[A-Za-z]+)(?<n>\d+)?)"s)});
        REQUIRE(result->is<Map>());
        const auto& result_map = result->raw_get<Map>();

        auto get_field = [&](const Map& map, std::string_view key) {
            auto it = map.find(Value::create(std::string{key}));
            REQUIRE(it != map.end());
            return it->second;
        };

        auto matches_val = get_field(result_map, "matches");
        REQUIRE(matches_val->is<Array>());
        const auto& matches = matches_val->raw_get<Array>();
        REQUIRE(matches.size() == 2);

        auto match0 = matches.at(0);
        REQUIRE(match0->is<Map>());
        const auto& match0_map = match0->raw_get<Map>();
        auto named0 = get_field(match0_map, "named");
        REQUIRE(named0->is<Map>());
        const auto& named0_map = named0->raw_get<Map>();

        auto k0 = get_field(named0_map, "k");
        auto n0 = get_field(named0_map, "n");
        REQUIRE(k0->is<Map>());
        REQUIRE(n0->is<Map>());
        const auto& k0_map = k0->raw_get<Map>();
        const auto& n0_map = n0->raw_get<Map>();
        CHECK(get_field(k0_map, "matched")->get<Bool>().value());
        CHECK(get_field(k0_map, "value")->raw_get<String>() == "foo");
        CHECK_FALSE(get_field(n0_map, "matched")->get<Bool>().value());
        CHECK(get_field(n0_map, "value")->is<Null>());

        auto groups0 = get_field(match0_map, "groups");
        REQUIRE(groups0->is<Array>());
        const auto& groups0_arr = groups0->raw_get<Array>();
        REQUIRE(groups0_arr.size() == 3);
        const auto& group2_map = groups0_arr.at(2)->raw_get<Map>();
        CHECK_FALSE(get_field(group2_map, "matched")->get<Bool>().value());
        CHECK(get_field(group2_map, "value")->is<Null>());

        auto match1 = matches.at(1);
        REQUIRE(match1->is<Map>());
        const auto& match1_map = match1->raw_get<Map>();
        auto named1 = get_field(match1_map, "named");
        REQUIRE(named1->is<Map>());
        const auto& named1_map = named1->raw_get<Map>();
        auto k1 = get_field(named1_map, "k");
        auto n1 = get_field(named1_map, "n");
        REQUIRE(k1->is<Map>());
        REQUIRE(n1->is<Map>());
        const auto& k1_map = k1->raw_get<Map>();
        const auto& n1_map = n1->raw_get<Map>();
        CHECK(get_field(k1_map, "matched")->get<Bool>().value());
        CHECK(get_field(k1_map, "value")->raw_get<String>() == "bar");
        CHECK(get_field(n1_map, "matched")->get<Bool>().value());
        CHECK(get_field(n1_map, "value")->raw_get<String>() == "1");
    }
}
