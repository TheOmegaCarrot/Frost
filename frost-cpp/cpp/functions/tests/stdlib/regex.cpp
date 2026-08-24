#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/builtins-common.hpp>
#include <frost/stdlib.hpp>
#include <frost/value.hpp>

using namespace frst;

using namespace std::literals;

using namespace Catch::Matchers;

namespace
{

Map regex_module()
{
    Stdlib_Registry_Builder builder;
    register_module_regex(builder);
    auto registry = std::move(builder).build();
    auto module = registry.lookup_module("std.regex");
    REQUIRE(module.has_value());
    REQUIRE(module.value()->is<Map>());
    return module.value()->raw_get<Map>();
}

Function lookup(const Map& mod, const std::string& name)
{
    auto key = Value::create(String{name});
    auto it = mod.find(key);
    REQUIRE(it != mod.end());
    REQUIRE(it->second->is<Function>());
    return it->second->raw_get<Function>();
}

} // namespace

TEST_CASE("std.regex")
{
    auto mod = regex_module();

    const std::vector<std::string> names{"matches", "contains"};
    const std::vector<std::string> replace_names{"replace", "replace_first"};
    const std::vector<std::string> binary_names{"scan_matches", "split"};

    auto get_re_fn = [&](const std::string& name) {
        return lookup(mod, name);
    };

    SECTION("Registered in module")
    {
        CHECK(mod.size() == 7);

        for (const auto& name : names)
            REQUIRE(get_re_fn(name));
        for (const auto& name : replace_names)
            REQUIRE(get_re_fn(name));
        for (const auto& name : binary_names)
            REQUIRE(get_re_fn(name));
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

    SECTION("Ternary arity (replace, replace_first)")
    {
        for (const auto& name : replace_names)
        {
            DYNAMIC_SECTION(name << " arity")
            {
                auto fn = get_re_fn(name);

                CHECK_THROWS_WITH(
                    fn->call({Value::create("a"s), Value::create("b"s)}),
                    ContainsSubstring("insufficient arguments")
                        && ContainsSubstring("requires at least 3"));

                CHECK_THROWS_WITH(
                    fn->call({Value::create("a"s), Value::create("b"s),
                              Value::create("c"s), Value::create("d"s)}),
                    ContainsSubstring("too many arguments")
                        && ContainsSubstring("no more than 3"));
            }
        }
    }

    SECTION("Binary arity (scan_matches, split)")
    {
        for (const auto& name : binary_names)
        {
            DYNAMIC_SECTION(name << " arity")
            {
                auto fn = get_re_fn(name);

                CHECK_THROWS_WITH(
                    fn->call({Value::create("a"s)}),
                    ContainsSubstring("insufficient arguments")
                        && ContainsSubstring("requires at least 2"));

                CHECK_THROWS_WITH(
                    fn->call({Value::create("a"s), Value::create("b"s),
                              Value::create("c"s)}),
                    ContainsSubstring("too many arguments")
                        && ContainsSubstring("no more than 2"));
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
                const auto first_fn =
                    ContainsSubstring(std::string{"Function regex."} + name);
                const auto first_string = ContainsSubstring("String");
                const auto first_type =
                    EndsWith(std::string{bad_first->type_name()});
                CHECK_THROWS_WITH(fn->call({bad_first, good}),
                                  first_fn && first_string && first_type);

                const auto second_fn =
                    ContainsSubstring(std::string{"Function regex."} + name);
                const auto second_string = ContainsSubstring("String");
                const auto second_type =
                    EndsWith(std::string{bad_second->type_name()});
                CHECK_THROWS_WITH(fn->call({good, bad_second}),
                                  second_fn && second_string && second_type);
            }
        }
    }

    SECTION("Ternary type errors (replace, replace_first)")
    {
        auto bad = Value::create(1_f);
        auto good = Value::create("a"s);

        for (const auto& name : replace_names)
        {
            DYNAMIC_SECTION(name << " type errors")
            {
                auto fn = get_re_fn(name);
                CHECK_THROWS_WITH(fn->call({bad, good, good}),
                                  ContainsSubstring("regex." + name)
                                      && ContainsSubstring("String")
                                      && ContainsSubstring("got Int"));
                CHECK_THROWS_WITH(fn->call({good, bad, good}),
                                  ContainsSubstring("regex." + name)
                                      && ContainsSubstring("String")
                                      && ContainsSubstring("got Int"));
                CHECK_THROWS_WITH(fn->call({good, good, bad}),
                                  ContainsSubstring("regex." + name)
                                      && ContainsSubstring("String")
                                      && ContainsSubstring("got Int"));
            }
        }
    }

    SECTION("Binary type errors (scan_matches, split)")
    {
        auto bad = Value::create(1_f);
        auto good = Value::create("a"s);

        for (const auto& name : binary_names)
        {
            DYNAMIC_SECTION(name << " type errors")
            {
                auto fn = get_re_fn(name);
                CHECK_THROWS_WITH(fn->call({bad, good}),
                                  ContainsSubstring("regex." + name)
                                      && ContainsSubstring("String")
                                      && ContainsSubstring("got Int"));
                CHECK_THROWS_WITH(fn->call({good, bad}),
                                  ContainsSubstring("regex." + name)
                                      && ContainsSubstring("String")
                                      && ContainsSubstring("got Int"));
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

        for (const auto& name : replace_names)
        {
            DYNAMIC_SECTION(name << " regex error")
            {
                auto fn = get_re_fn(name);
                CHECK_THROWS_MATCHES(
                    fn->call({target, bad_re, Value::create("x"s)}),
                    Frost_User_Error,
                    MessageMatches(StartsWith("Regex error: ")));
            }
        }

        for (const auto& name : binary_names)
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

    SECTION("Replace first success")
    {
        auto target = Value::create("a1b2c3"s);
        auto pattern = Value::create("[0-9]"s);
        auto replacement = Value::create("X"s);

        CHECK(get_re_fn("replace_first")
                  ->call({target, pattern, replacement})
                  ->raw_get<String>()
              == "aXb2c3");

        // No match leaves string unchanged
        auto no_match = Value::create("abc"s);
        CHECK(get_re_fn("replace_first")
                  ->call({no_match, pattern, replacement})
                  ->raw_get<String>()
              == "abc");

        // Capture group substitution works
        CHECK(get_re_fn("replace_first")
                  ->call({Value::create("foo bar"s), Value::create(R"((\w+))"s),
                          Value::create("<$1>"s)})
                  ->raw_get<String>()
              == "<foo> bar");
    }

    SECTION("Replace with callback")
    {
        auto fn = get_re_fn("replace_with");

        // Arity
        CHECK_THROWS_WITH(fn->call({Value::create("a"s), Value::create("b"s)}),
                          ContainsSubstring("insufficient arguments")
                              && ContainsSubstring("requires at least 3"));

        // Type errors
        CHECK_THROWS_WITH(
            fn->call({Value::create(1_f), Value::create("."s),
                      Value::create(Function{std::make_shared<Builtin>(
                          [](builtin_args_t) {
                              return Value::null();
                          },
                          "dummy")})}),
            ContainsSubstring("regex.replace_with")
                && ContainsSubstring("got Int"));

        // Basic callback
        auto upper = system_closure([](builtin_args_t args) {
            const auto& s = args.at(0)->raw_get<String>();
            std::string result;
            for (char c : s)
                result += static_cast<char>(
                    std::toupper(static_cast<unsigned char>(c)));
            return Value::create(std::move(result));
        });

        CHECK(fn->call({Value::create("hello world"s), Value::create(R"(\w+)"s),
                        upper})
                  ->raw_get<String>()
              == "HELLO WORLD");

        // Callback return value is to_string-ed
        auto times_ten = system_closure([](builtin_args_t args) {
            auto n = std::stoll(args.at(0)->raw_get<String>());
            return Value::create(Int{n * 10});
        });

        CHECK(fn->call({Value::create("a1b2"s), Value::create(R"(\d)"s),
                        times_ten})
                  ->raw_get<String>()
              == "a10b20");

        // No match leaves string unchanged
        CHECK(fn->call({Value::create("hello"s), Value::create("z+"s), upper})
                  ->raw_get<String>()
              == "hello");

        // Regex error
        CHECK_THROWS_MATCHES(
            fn->call({Value::create("x"s), Value::create("("s), upper}),
            Frost_User_Error, MessageMatches(StartsWith("Regex error: ")));
    }

    SECTION("Split")
    {
        auto fn = get_re_fn("split");

        // Basic split
        auto result =
            fn->call({Value::create("one,two,three"s), Value::create(","s)});
        REQUIRE(result->is<Array>());
        const auto& arr = result->raw_get<Array>();
        REQUIRE(arr.size() == 3);
        CHECK(arr[0]->raw_get<String>() == "one");
        CHECK(arr[1]->raw_get<String>() == "two");
        CHECK(arr[2]->raw_get<String>() == "three");

        // Multi-char delimiter
        auto result2 =
            fn->call({Value::create("a,,b,,,c"s), Value::create(",+"s)});
        REQUIRE(result2->is<Array>());
        const auto& arr2 = result2->raw_get<Array>();
        REQUIRE(arr2.size() == 3);
        CHECK(arr2[0]->raw_get<String>() == "a");
        CHECK(arr2[1]->raw_get<String>() == "b");
        CHECK(arr2[2]->raw_get<String>() == "c");

        // No match returns single-element array
        auto result3 = fn->call({Value::create("hello"s), Value::create("x"s)});
        REQUIRE(result3->is<Array>());
        CHECK(result3->raw_get<Array>().size() == 1);
        CHECK(result3->raw_get<Array>()[0]->raw_get<String>() == "hello");

        // Empty string produces empty array
        auto result4 = fn->call({Value::create(""s), Value::create(","s)});
        REQUIRE(result4->is<Array>());
        CHECK(result4->raw_get<Array>().empty());
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
        auto fn = get_re_fn("scan_matches");
        auto result = fn->call({Value::create("abc"s), Value::create("z+"s)});
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
        auto fn = get_re_fn("scan_matches");
        auto result = fn->call({Value::create("foo=bar beep=boop"s),
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
        auto fn = get_re_fn("scan_matches");
        auto result = fn->call({Value::create("foo=bar beep=boop"s),
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
        auto fn = get_re_fn("scan_matches");
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
