#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/stdlib.hpp>
#include <frost/value.hpp>

using namespace frst;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;
using Catch::Matchers::MessageMatches;

namespace
{

Map cli_module()
{
    Stdlib_Registry_Builder builder;
    register_module_cli(builder);
    auto registry = std::move(builder).build();
    auto module = registry.lookup_module("std.cli");
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

// Helper to build an args Array
Array make_args(std::initializer_list<std::string> strs)
{
    Array arr;
    for (const auto& s : strs)
        arr.push_back(Value::create(String{s}));
    return arr;
}

// Helper to look up a string key in a Map
Value_Ptr get(const Map& m, const std::string& key)
{
    auto it = m.find(Value::create(String{key}));
    REQUIRE(it != m.end());
    return it->second;
}

} // namespace

TEST_CASE("std.cli")
{
    auto mod = cli_module();

    SECTION("Registered")
    {
        CHECK(mod.size() == 2);
        lookup(mod, "parse");
        lookup(mod, "prompt");
    }
}

TEST_CASE("cli.parse arity and types")
{
    auto mod = cli_module();
    auto parse = lookup(mod, "parse");

    SECTION("Too few args")
    {
        CHECK_THROWS_MATCHES(
            parse->call({}), Frost_User_Error,
            MessageMatches(ContainsSubstring("insufficient arguments")));
    }

    SECTION("First arg not Array")
    {
        CHECK_THROWS_MATCHES(
            parse->call(
                {Value::create("x"s), Value::create(Value::trusted, Map{})}),
            Frost_User_Error, MessageMatches(ContainsSubstring("Array")));
    }

    SECTION("Second arg not Map")
    {
        CHECK_THROWS_MATCHES(
            parse->call({Value::create(Array{}), Value::create("x"s)}),
            Frost_User_Error, MessageMatches(ContainsSubstring("Map")));
    }

    SECTION("Empty args")
    {
        CHECK_THROWS_MATCHES(
            parse->call(
                {Value::create(Array{}), Value::create(Value::trusted, Map{})}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("non-empty")));
    }

    SECTION("Non-string in args")
    {
        Array bad_args = {Value::create(42_f)};
        CHECK_THROWS_MATCHES(
            parse->call({Value::create(std::move(bad_args)),
                         Value::create(Value::trusted, Map{})}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("args[0]")
                           && ContainsSubstring("String")));
    }
}

TEST_CASE("cli.parse spec validation")
{
    auto mod = cli_module();
    auto parse = lookup(mod, "parse");

    auto args = Value::create(make_args({"script.frst"}));

    auto call_with_spec = [&](Map spec) {
        return parse->call(
            {args, Value::create(Value::trusted, std::move(spec))});
    };

    SECTION("Unknown top-level key")
    {
        CHECK_THROWS_MATCHES(
            call_with_spec(Map{{"bogus"_s, Value::null()}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("unknown spec key")));
    }

    SECTION("name must be String")
    {
        CHECK_THROWS_MATCHES(
            call_with_spec(Map{{"name"_s, Value::create(42_f)}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("name")
                           && ContainsSubstring("String")));
    }

    SECTION("Flag with unknown key")
    {
        Map flag_spec = {{"required"_s, Value::create(Bool{true})}};
        Map flags = {
            {"verbose"_s, Value::create(Value::trusted, std::move(flag_spec))}};
        CHECK_THROWS_MATCHES(
            call_with_spec(Map{
                {"flags"_s, Value::create(Value::trusted, std::move(flags))}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("unknown key")
                           && ContainsSubstring("required")));
    }

    SECTION("Duplicate short name")
    {
        Map f1 = {{"short"_s, Value::create("v"s)}};
        Map f2 = {{"short"_s, Value::create("v"s)}};
        Map flags = {
            {"verbose"_s, Value::create(Value::trusted, std::move(f1))},
            {"very"_s, Value::create(Value::trusted, std::move(f2))}};
        CHECK_THROWS_MATCHES(
            call_with_spec(Map{
                {"flags"_s, Value::create(Value::trusted, std::move(flags))}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("duplicate short")));
    }

    SECTION("Duplicate long name across flags and options")
    {
        Map flag_spec = {};
        Map flags = {
            {"name"_s, Value::create(Value::trusted, std::move(flag_spec))}};
        Map opt_spec = {};
        Map options = {
            {"name"_s, Value::create(Value::trusted, std::move(opt_spec))}};
        CHECK_THROWS_MATCHES(
            call_with_spec(Map{
                {"flags"_s, Value::create(Value::trusted, std::move(flags))},
                {"options"_s,
                 Value::create(Value::trusted, std::move(options))}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("duplicate name")));
    }

    SECTION("Positional entry missing name")
    {
        Array pos = {Value::create(Value::trusted, Map{})};
        CHECK_THROWS_MATCHES(
            call_with_spec(
                Map{{"positional"_s, Value::create(std::move(pos))}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("name is required")));
    }

    SECTION("description must be String")
    {
        CHECK_THROWS_MATCHES(
            call_with_spec(Map{{"description"_s, Value::create(42_f)}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("description")
                           && ContainsSubstring("String")));
    }

    SECTION("help must be String")
    {
        CHECK_THROWS_MATCHES(
            call_with_spec(Map{{"help"_s, Value::create(42_f)}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("help")
                           && ContainsSubstring("String")));
    }

    SECTION("flags must be a Map")
    {
        CHECK_THROWS_MATCHES(
            call_with_spec(Map{{"flags"_s, Value::create("bogus"s)}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("flags")
                           && ContainsSubstring("Map")));
    }

    SECTION("options must be a Map")
    {
        CHECK_THROWS_MATCHES(
            call_with_spec(Map{{"options"_s, Value::create("bogus"s)}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("options")
                           && ContainsSubstring("Map")));
    }

    SECTION("Flag sub-spec must be a Map")
    {
        Map flags = {{"verbose"_s, Value::create(42_f)}};
        CHECK_THROWS_MATCHES(
            call_with_spec(Map{
                {"flags"_s, Value::create(Value::trusted, std::move(flags))}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("flag 'verbose'")
                           && ContainsSubstring("Map")));
    }

    SECTION("Option sub-spec must be a Map")
    {
        Map options = {{"env"_s, Value::create(42_f)}};
        CHECK_THROWS_MATCHES(
            call_with_spec(
                Map{{"options"_s,
                     Value::create(Value::trusted, std::move(options))}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("option 'env'")
                           && ContainsSubstring("Map")));
    }

    SECTION("Flag short not single character")
    {
        Map flag = {{"short"_s, Value::create("vv"s)}};
        Map flags = {
            {"verbose"_s, Value::create(Value::trusted, std::move(flag))}};
        CHECK_THROWS_MATCHES(
            call_with_spec(Map{
                {"flags"_s, Value::create(Value::trusted, std::move(flags))}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("single character")));
    }

    SECTION("Flag short not String")
    {
        Map flag = {{"short"_s, Value::create(42_f)}};
        Map flags = {
            {"verbose"_s, Value::create(Value::trusted, std::move(flag))}};
        CHECK_THROWS_MATCHES(
            call_with_spec(Map{
                {"flags"_s, Value::create(Value::trusted, std::move(flags))}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("short")
                           && ContainsSubstring("String")));
    }

    SECTION("Flag description not String")
    {
        Map flag = {{"description"_s, Value::create(42_f)}};
        Map flags = {
            {"verbose"_s, Value::create(Value::trusted, std::move(flag))}};
        CHECK_THROWS_MATCHES(
            call_with_spec(Map{
                {"flags"_s, Value::create(Value::trusted, std::move(flags))}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("description")
                           && ContainsSubstring("String")));
    }

    SECTION("Option short not single character")
    {
        Map opt = {{"short"_s, Value::create("ee"s)}};
        Map options = {
            {"env"_s, Value::create(Value::trusted, std::move(opt))}};
        CHECK_THROWS_MATCHES(
            call_with_spec(
                Map{{"options"_s,
                     Value::create(Value::trusted, std::move(options))}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("single character")));
    }

    SECTION("Option short not String")
    {
        Map opt = {{"short"_s, Value::create(42_f)}};
        Map options = {
            {"env"_s, Value::create(Value::trusted, std::move(opt))}};
        CHECK_THROWS_MATCHES(
            call_with_spec(
                Map{{"options"_s,
                     Value::create(Value::trusted, std::move(options))}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("short")
                           && ContainsSubstring("String")));
    }

    SECTION("Option required not Bool")
    {
        Map opt = {{"required"_s, Value::create("yes"s)}};
        Map options = {
            {"env"_s, Value::create(Value::trusted, std::move(opt))}};
        CHECK_THROWS_MATCHES(
            call_with_spec(
                Map{{"options"_s,
                     Value::create(Value::trusted, std::move(options))}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("required")
                           && ContainsSubstring("Bool")));
    }

    SECTION("Option default not String")
    {
        Map opt = {{"default"_s, Value::create(42_f)}};
        Map options = {
            {"env"_s, Value::create(Value::trusted, std::move(opt))}};
        CHECK_THROWS_MATCHES(
            call_with_spec(
                Map{{"options"_s,
                     Value::create(Value::trusted, std::move(options))}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("default")
                           && ContainsSubstring("String")));
    }

    SECTION("Option repeatable not Bool")
    {
        Map opt = {{"repeatable"_s, Value::create("yes"s)}};
        Map options = {
            {"env"_s, Value::create(Value::trusted, std::move(opt))}};
        CHECK_THROWS_MATCHES(
            call_with_spec(
                Map{{"options"_s,
                     Value::create(Value::trusted, std::move(options))}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("repeatable")
                           && ContainsSubstring("Bool")));
    }

    SECTION("Option description not String")
    {
        Map opt = {{"description"_s, Value::create(42_f)}};
        Map options = {
            {"env"_s, Value::create(Value::trusted, std::move(opt))}};
        CHECK_THROWS_MATCHES(
            call_with_spec(
                Map{{"options"_s,
                     Value::create(Value::trusted, std::move(options))}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("description")
                           && ContainsSubstring("String")));
    }

    SECTION("Option with unknown key")
    {
        Map opt = {{"bogus"_s, Value::create(Bool{true})}};
        Map options = {
            {"env"_s, Value::create(Value::trusted, std::move(opt))}};
        CHECK_THROWS_MATCHES(
            call_with_spec(
                Map{{"options"_s,
                     Value::create(Value::trusted, std::move(options))}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("unknown key")
                           && ContainsSubstring("bogus")));
    }

    SECTION("Non-String key in outer spec Map")
    {
        CHECK_THROWS_MATCHES(
            call_with_spec(Map{{Value::create(42_f), Value::null()}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("spec")
                           && ContainsSubstring("String")));
    }

    SECTION("positional as non-Bool non-Array")
    {
        CHECK_THROWS_MATCHES(
            call_with_spec(Map{{"positional"_s, Value::create(42_f)}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("positional")));
    }

    SECTION("positional as false means no positionals")
    {
        // Equivalent to omitting `positional` or using `[]`.
        auto result = parse->call(
            {Value::create(make_args({"s.frst"})),
             Value::create(Value::trusted,
                           Map{{"positional"_s, Value::create(Bool{false})}})});
        REQUIRE(result->is<Map>());
        CHECK(get(result->raw_get<Map>(), "positional")
                  ->raw_get<Array>()
                  .empty());
    }

    SECTION("positional as false rejects positional args")
    {
        CHECK_THROWS_MATCHES(
            parse->call({Value::create(make_args({"s.frst", "extra"})),
                         Value::create(Value::trusted,
                                       Map{{"positional"_s,
                                            Value::create(Bool{false})}})}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("unexpected positional")));
    }

    SECTION("Positional entry not a Map")
    {
        Array pos = {Value::create(42_f)};
        CHECK_THROWS_MATCHES(
            call_with_spec(
                Map{{"positional"_s, Value::create(std::move(pos))}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("positional[0]")
                           && ContainsSubstring("Map")));
    }

    SECTION("Positional entry with unknown key")
    {
        Map entry = {{"name"_s, Value::create("svc"s)},
                     {"bogus"_s, Value::create("x"s)}};
        Array pos = {Value::create(Value::trusted, std::move(entry))};
        CHECK_THROWS_MATCHES(
            call_with_spec(
                Map{{"positional"_s, Value::create(std::move(pos))}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("unknown key")
                           && ContainsSubstring("bogus")));
    }

    SECTION("Positional entry with non-String name")
    {
        Map entry = {{"name"_s, Value::create(42_f)}};
        Array pos = {Value::create(Value::trusted, std::move(entry))};
        CHECK_THROWS_MATCHES(
            call_with_spec(
                Map{{"positional"_s, Value::create(std::move(pos))}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("name")
                           && ContainsSubstring("String")));
    }

    SECTION("Positional entry with non-String description")
    {
        Map entry = {{"name"_s, Value::create("svc"s)},
                     {"description"_s, Value::create(42_f)}};
        Array pos = {Value::create(Value::trusted, std::move(entry))};
        CHECK_THROWS_MATCHES(
            call_with_spec(
                Map{{"positional"_s, Value::create(std::move(pos))}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("description")
                           && ContainsSubstring("String")));
    }

    SECTION("Empty positional Array accepts zero args")
    {
        // Not an error; positional: [] means no positionals expected
        auto result = parse->call(
            {Value::create(make_args({"s.frst"})),
             Value::create(Value::trusted,
                           Map{{"positional"_s, Value::create(Array{})}})});
        REQUIRE(result->is<Map>());
        CHECK(get(result->raw_get<Map>(), "positional")
                  ->raw_get<Array>()
                  .empty());
    }

    SECTION("Non-String key in flags Map")
    {
        Map flags = {
            {Value::create(42_f), Value::create(Value::trusted, Map{})}};
        CHECK_THROWS_MATCHES(
            call_with_spec(Map{
                {"flags"_s, Value::create(Value::trusted, std::move(flags))}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("flags")
                           && ContainsSubstring("String")));
    }

    SECTION("Non-String key in options Map")
    {
        Map options = {
            {Value::create(42_f), Value::create(Value::trusted, Map{})}};
        CHECK_THROWS_MATCHES(
            call_with_spec(
                Map{{"options"_s,
                     Value::create(Value::trusted, std::move(options))}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("options")
                           && ContainsSubstring("String")));
    }

    SECTION("Option with both required and default is an error")
    {
        Map opt = {{"required"_s, Value::create(Bool{true})},
                   {"default"_s, Value::create("x"s)}};
        Map options = {
            {"env"_s, Value::create(Value::trusted, std::move(opt))}};
        CHECK_THROWS_MATCHES(
            call_with_spec(
                Map{{"options"_s,
                     Value::create(Value::trusted, std::move(options))}}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("required")
                           && ContainsSubstring("default")
                           && ContainsSubstring("mutually exclusive")));
    }
}

TEST_CASE("cli.parse successful parsing")
{
    auto mod = cli_module();
    auto parse = lookup(mod, "parse");

    auto do_parse = [&](std::initializer_list<std::string> argv, Map spec) {
        auto result =
            parse->call({Value::create(make_args(argv)),
                         Value::create(Value::trusted, std::move(spec))});
        REQUIRE(result->is<Map>());
        return result->raw_get<Map>();
    };

    SECTION("Empty spec, no args")
    {
        auto result = do_parse({"script.frst"}, {});
        auto flags = get(result, "flags");
        CHECK(flags->raw_get<Map>().empty());
    }

    SECTION("Flag set with long form")
    {
        Map flag_spec = {};
        Map flags = {
            {"verbose"_s, Value::create(Value::trusted, std::move(flag_spec))}};
        auto result = do_parse(
            {"s.frst", "--verbose"},
            Map{{"flags"_s, Value::create(Value::trusted, std::move(flags))}});
        auto flags_result = get(result, "flags")->raw_get<Map>();
        CHECK(get(flags_result, "verbose")->raw_get<Bool>() == true);
    }

    SECTION("Flag not set")
    {
        Map flag_spec = {};
        Map flags = {
            {"verbose"_s, Value::create(Value::trusted, std::move(flag_spec))}};
        auto result = do_parse(
            {"s.frst"},
            Map{{"flags"_s, Value::create(Value::trusted, std::move(flags))}});
        auto flags_result = get(result, "flags")->raw_get<Map>();
        CHECK(get(flags_result, "verbose")->raw_get<Bool>() == false);
    }

    SECTION("Short flag")
    {
        Map flag_spec = {{"short"_s, Value::create("v"s)}};
        Map flags = {
            {"verbose"_s, Value::create(Value::trusted, std::move(flag_spec))}};
        auto result = do_parse(
            {"s.frst", "-v"},
            Map{{"flags"_s, Value::create(Value::trusted, std::move(flags))}});
        auto flags_result = get(result, "flags")->raw_get<Map>();
        CHECK(get(flags_result, "verbose")->raw_get<Bool>() == true);
    }

    SECTION("Flag bundle")
    {
        Map f1 = {{"short"_s, Value::create("v"s)}};
        Map f2 = {{"short"_s, Value::create("d"s)}};
        Map flags = {
            {"verbose"_s, Value::create(Value::trusted, std::move(f1))},
            {"debug"_s, Value::create(Value::trusted, std::move(f2))}};
        auto result = do_parse(
            {"s.frst", "-vd"},
            Map{{"flags"_s, Value::create(Value::trusted, std::move(flags))}});
        auto flags_result = get(result, "flags")->raw_get<Map>();
        CHECK(get(flags_result, "verbose")->raw_get<Bool>() == true);
        CHECK(get(flags_result, "debug")->raw_get<Bool>() == true);
    }

    SECTION("Option with value")
    {
        Map opt_spec = {};
        Map options = {
            {"env"_s, Value::create(Value::trusted, std::move(opt_spec))}};
        auto result =
            do_parse({"s.frst", "--env", "prod"},
                     Map{{"options"_s,
                          Value::create(Value::trusted, std::move(options))}});
        auto opts = get(result, "options")->raw_get<Map>();
        CHECK(get(opts, "env")->raw_get<String>() == "prod");
    }

    SECTION("Option default")
    {
        Map opt_spec = {{"default"_s, Value::create("3"s)}};
        Map options = {
            {"retries"_s, Value::create(Value::trusted, std::move(opt_spec))}};
        auto result =
            do_parse({"s.frst"},
                     Map{{"options"_s,
                          Value::create(Value::trusted, std::move(options))}});
        auto opts = get(result, "options")->raw_get<Map>();
        CHECK(get(opts, "retries")->raw_get<String>() == "3");
    }

    SECTION("Option null when absent")
    {
        Map opt_spec = {};
        Map options = {
            {"env"_s, Value::create(Value::trusted, std::move(opt_spec))}};
        auto result =
            do_parse({"s.frst"},
                     Map{{"options"_s,
                          Value::create(Value::trusted, std::move(options))}});
        auto opts = get(result, "options")->raw_get<Map>();
        CHECK(get(opts, "env")->is<Null>());
    }

    SECTION("Repeatable option")
    {
        Map opt_spec = {{"repeatable"_s, Value::create(Bool{true})}};
        Map options = {
            {"tag"_s, Value::create(Value::trusted, std::move(opt_spec))}};
        auto result =
            do_parse({"s.frst", "--tag", "v1", "--tag", "v2"},
                     Map{{"options"_s,
                          Value::create(Value::trusted, std::move(options))}});
        auto opts = get(result, "options")->raw_get<Map>();
        auto tags = get(opts, "tag")->raw_get<Array>();
        REQUIRE(tags.size() == 2);
        CHECK(tags[0]->raw_get<String>() == "v1");
        CHECK(tags[1]->raw_get<String>() == "v2");
    }

    SECTION("Repeatable option empty produces empty array")
    {
        Map opt_spec = {{"repeatable"_s, Value::create(Bool{true})}};
        Map options = {
            {"tag"_s, Value::create(Value::trusted, std::move(opt_spec))}};
        auto result =
            do_parse({"s.frst"},
                     Map{{"options"_s,
                          Value::create(Value::trusted, std::move(options))}});
        auto opts = get(result, "options")->raw_get<Map>();
        CHECK(get(opts, "tag")->raw_get<Array>().empty());
    }

    SECTION("Positional args")
    {
        Map pos_entry = {{"name"_s, Value::create("service"s)}};
        Array pos = {Value::create(Value::trusted, std::move(pos_entry))};
        auto result =
            do_parse({"s.frst", "web-api"},
                     Map{{"positional"_s, Value::create(std::move(pos))}});
        auto positionals = get(result, "positional")->raw_get<Array>();
        REQUIRE(positionals.size() == 1);
        CHECK(positionals[0]->raw_get<String>() == "web-api");
    }

    SECTION("Double dash stops flag parsing")
    {
        Map flag_spec = {};
        Map flags = {
            {"verbose"_s, Value::create(Value::trusted, std::move(flag_spec))}};
        auto result = do_parse(
            {"s.frst", "--", "--verbose"},
            Map{{"flags"_s, Value::create(Value::trusted, std::move(flags))},
                {"positional"_s, Value::create(Bool{true})}});
        auto flags_result = get(result, "flags")->raw_get<Map>();
        CHECK(get(flags_result, "verbose")->raw_get<Bool>() == false);
        auto positionals = get(result, "positional")->raw_get<Array>();
        REQUIRE(positionals.size() == 1);
        CHECK(positionals[0]->raw_get<String>() == "--verbose");
    }

    SECTION("Positional true: raw collect")
    {
        auto result =
            do_parse({"s.frst", "a", "b", "c"},
                     Map{{"positional"_s, Value::create(Bool{true})}});
        auto positionals = get(result, "positional")->raw_get<Array>();
        REQUIRE(positionals.size() == 3);
    }

    SECTION("Result contains help key")
    {
        auto result = do_parse({"s.frst"}, {});
        auto help = get(result, "help");
        REQUIRE(help->is<String>());
        CHECK(not help->raw_get<String>().empty());
    }

    SECTION("Custom help text is returned verbatim")
    {
        auto result = do_parse(
            {"s.frst"}, Map{{"help"_s, Value::create("my custom help"s)}});
        CHECK(get(result, "help")->raw_get<String>() == "my custom help");
    }

    SECTION("Help contains spec name")
    {
        auto result =
            do_parse({"s.frst"}, Map{{"name"_s, Value::create("deploy"s)}});
        CHECK(get(result, "help")->raw_get<String>().contains("deploy"));
    }

    SECTION("Help contains description")
    {
        auto result = do_parse(
            {"s.frst"},
            Map{{"name"_s, Value::create("deploy"s)},
                {"description"_s, Value::create("Deploy a service"s)}});
        CHECK(get(result, "help")
                  ->raw_get<String>()
                  .contains("Deploy a service"));
    }

    SECTION("Help falls back to args[0] when name absent")
    {
        auto result = do_parse({"mytool"}, {});
        CHECK(get(result, "help")->raw_get<String>().contains("mytool"));
    }

    SECTION("Help contains flag descriptions")
    {
        Map flag = {{"short"_s, Value::create("v"s)},
                    {"description"_s, Value::create("verbose mode"s)}};
        Map flags = {
            {"verbose"_s, Value::create(Value::trusted, std::move(flag))}};
        auto result = do_parse(
            {"s.frst"},
            Map{{"flags"_s, Value::create(Value::trusted, std::move(flags))}});
        auto help_str = get(result, "help")->raw_get<String>();
        CHECK(help_str.contains("-v"));
        CHECK(help_str.contains("--verbose"));
        CHECK(help_str.contains("verbose mode"));
    }

    SECTION("Help contains option descriptions with required marker")
    {
        Map opt = {{"short"_s, Value::create("e"s)},
                   {"required"_s, Value::create(Bool{true})},
                   {"description"_s, Value::create("target env"s)}};
        Map options = {
            {"env"_s, Value::create(Value::trusted, std::move(opt))}};
        auto result =
            do_parse({"s.frst", "--env", "x"},
                     Map{{"options"_s,
                          Value::create(Value::trusted, std::move(options))}});
        auto help_str = get(result, "help")->raw_get<String>();
        CHECK(help_str.contains("--env"));
        CHECK(help_str.contains("target env"));
        CHECK(help_str.contains("required"));
    }

    SECTION("Help contains default marker")
    {
        Map opt = {{"default"_s, Value::create("3"s)}};
        Map options = {
            {"retries"_s, Value::create(Value::trusted, std::move(opt))}};
        auto result =
            do_parse({"s.frst"},
                     Map{{"options"_s,
                          Value::create(Value::trusted, std::move(options))}});
        CHECK(get(result, "help")->raw_get<String>().contains("default: 3"));
    }

    SECTION("Help contains repeatable marker")
    {
        Map opt = {{"repeatable"_s, Value::create(Bool{true})}};
        Map options = {
            {"tag"_s, Value::create(Value::trusted, std::move(opt))}};
        auto result =
            do_parse({"s.frst"},
                     Map{{"options"_s,
                          Value::create(Value::trusted, std::move(options))}});
        CHECK(get(result, "help")->raw_get<String>().contains("repeatable"));
    }

    SECTION("Help contains positional names")
    {
        Map entry = {{"name"_s, Value::create("service"s)},
                     {"description"_s, Value::create("the thing to deploy"s)}};
        Array pos = {Value::create(Value::trusted, std::move(entry))};
        auto result =
            do_parse({"s.frst", "web"},
                     Map{{"positional"_s, Value::create(std::move(pos))}});
        auto help_str = get(result, "help")->raw_get<String>();
        CHECK(help_str.contains("service"));
        CHECK(help_str.contains("the thing to deploy"));
    }

    SECTION("Help for raw collect positionals")
    {
        auto result = do_parse(
            {"s.frst"}, Map{{"positional"_s, Value::create(Bool{true})}});
        CHECK(get(result, "help")->raw_get<String>().contains("[args...]"));
    }

    SECTION("Long-form use of flag with short name")
    {
        Map flag = {{"short"_s, Value::create("v"s)}};
        Map flags = {
            {"verbose"_s, Value::create(Value::trusted, std::move(flag))}};
        auto result = do_parse(
            {"s.frst", "--verbose"},
            Map{{"flags"_s, Value::create(Value::trusted, std::move(flags))}});
        CHECK(get(get(result, "flags")->raw_get<Map>(), "verbose")
                  ->raw_get<Bool>()
              == true);
    }

    SECTION("Long-form use of option with short name")
    {
        Map opt = {{"short"_s, Value::create("e"s)}};
        Map options = {
            {"env"_s, Value::create(Value::trusted, std::move(opt))}};
        auto result =
            do_parse({"s.frst", "--env", "prod"},
                     Map{{"options"_s,
                          Value::create(Value::trusted, std::move(options))}});
        CHECK(get(get(result, "options")->raw_get<Map>(), "env")
                  ->raw_get<String>()
              == "prod");
    }

    SECTION("Flags and positionals interleaved")
    {
        Map flag = {};
        Map flags = {
            {"verbose"_s, Value::create(Value::trusted, std::move(flag))}};
        Map pos_entry = {{"name"_s, Value::create("svc"s)}};
        Array pos = {Value::create(Value::trusted, std::move(pos_entry))};
        auto result = do_parse(
            {"s.frst", "--verbose", "web-api"},
            Map{{"flags"_s, Value::create(Value::trusted, std::move(flags))},
                {"positional"_s, Value::create(std::move(pos))}});
        CHECK(get(get(result, "flags")->raw_get<Map>(), "verbose")
                  ->raw_get<Bool>()
              == true);
        auto positionals = get(result, "positional")->raw_get<Array>();
        REQUIRE(positionals.size() == 1);
        CHECK(positionals.at(0)->raw_get<String>() == "web-api");
    }

    SECTION("Bundle ending with value option")
    {
        Map f1 = {{"short"_s, Value::create("v"s)}};
        Map f2 = {{"short"_s, Value::create("d"s)}};
        Map flags = {
            {"verbose"_s, Value::create(Value::trusted, std::move(f1))},
            {"debug"_s, Value::create(Value::trusted, std::move(f2))}};
        Map opt = {{"short"_s, Value::create("e"s)}};
        Map options = {
            {"env"_s, Value::create(Value::trusted, std::move(opt))}};
        auto result = do_parse(
            {"s.frst", "-vde", "prod"},
            Map{{"flags"_s, Value::create(Value::trusted, std::move(flags))},
                {"options"_s,
                 Value::create(Value::trusted, std::move(options))}});
        auto flag_map = get(result, "flags")->raw_get<Map>();
        CHECK(get(flag_map, "verbose")->raw_get<Bool>() == true);
        CHECK(get(flag_map, "debug")->raw_get<Bool>() == true);
        CHECK(get(get(result, "options")->raw_get<Map>(), "env")
                  ->raw_get<String>()
              == "prod");
    }

    SECTION("Value-taking option not at end of bundle is an error")
    {
        Map f1 = {{"short"_s, Value::create("v"s)}};
        Map f2 = {{"short"_s, Value::create("d"s)}};
        Map flags = {
            {"verbose"_s, Value::create(Value::trusted, std::move(f1))},
            {"debug"_s, Value::create(Value::trusted, std::move(f2))}};
        Map opt = {{"short"_s, Value::create("e"s)}};
        Map options = {
            {"env"_s, Value::create(Value::trusted, std::move(opt))}};
        // -ved: e takes a value but is not last. Should error.
        CHECK_THROWS_MATCHES(
            parse->call(
                {Value::create(make_args({"s.frst", "-ved", "prod"})),
                 Value::create(
                     Value::trusted,
                     Map{{"flags"_s,
                          Value::create(Value::trusted, std::move(flags))},
                         {"options"_s, Value::create(Value::trusted,
                                                     std::move(options))}})}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("must be the last")
                           || ContainsSubstring("-e")));
    }

    SECTION("Short-form repeatable option")
    {
        Map opt = {{"short"_s, Value::create("t"s)},
                   {"repeatable"_s, Value::create(Bool{true})}};
        Map options = {
            {"tag"_s, Value::create(Value::trusted, std::move(opt))}};
        auto result =
            do_parse({"s.frst", "-t", "v1", "-t", "v2", "-t", "v3"},
                     Map{{"options"_s,
                          Value::create(Value::trusted, std::move(options))}});
        auto tags = get(get(result, "options")->raw_get<Map>(), "tag")
                        ->raw_get<Array>();
        REQUIRE(tags.size() == 3);
        CHECK(tags.at(0)->raw_get<String>() == "v1");
        CHECK(tags.at(1)->raw_get<String>() == "v2");
        CHECK(tags.at(2)->raw_get<String>() == "v3");
    }

    SECTION("Option value starting with dash")
    {
        Map opt = {};
        Map options = {
            {"env"_s, Value::create(Value::trusted, std::move(opt))}};
        auto result =
            do_parse({"s.frst", "--env", "-v"},
                     Map{{"options"_s,
                          Value::create(Value::trusted, std::move(options))}});
        CHECK(get(get(result, "options")->raw_get<Map>(), "env")
                  ->raw_get<String>()
              == "-v");
    }

    SECTION("Option consumes following arg even if it looks like a flag")
    {
        // GNU behavior: `--env --verbose` makes `--verbose` the value of
        // `--env`, even though `--verbose` is also a declared flag.
        Map flag = {};
        Map flags = {
            {"verbose"_s, Value::create(Value::trusted, std::move(flag))}};
        Map opt = {};
        Map options = {
            {"env"_s, Value::create(Value::trusted, std::move(opt))}};
        auto result = do_parse(
            {"s.frst", "--env", "--verbose"},
            Map{{"flags"_s, Value::create(Value::trusted, std::move(flags))},
                {"options"_s,
                 Value::create(Value::trusted, std::move(options))}});
        CHECK(get(get(result, "options")->raw_get<Map>(), "env")
                  ->raw_get<String>()
              == "--verbose");
        CHECK(get(get(result, "flags")->raw_get<Map>(), "verbose")
                  ->raw_get<Bool>()
              == false);
    }

    SECTION("Long option name with dashes")
    {
        Map flag = {};
        Map flags = {
            {"foo-bar"_s, Value::create(Value::trusted, std::move(flag))}};
        auto result = do_parse(
            {"s.frst", "--foo-bar"},
            Map{{"flags"_s, Value::create(Value::trusted, std::move(flags))}});
        CHECK(get(get(result, "flags")->raw_get<Map>(), "foo-bar")
                  ->raw_get<Bool>()
              == true);
    }

    SECTION("Empty string option value")
    {
        Map opt = {};
        Map options = {
            {"env"_s, Value::create(Value::trusted, std::move(opt))}};
        auto result =
            do_parse({"s.frst", "--env", ""},
                     Map{{"options"_s,
                          Value::create(Value::trusted, std::move(options))}});
        CHECK(get(get(result, "options")->raw_get<Map>(), "env")
                  ->raw_get<String>()
              == "");
    }

    SECTION("Required option when provided works")
    {
        Map opt = {{"required"_s, Value::create(Bool{true})}};
        Map options = {
            {"env"_s, Value::create(Value::trusted, std::move(opt))}};
        auto result =
            do_parse({"s.frst", "--env", "prod"},
                     Map{{"options"_s,
                          Value::create(Value::trusted, std::move(options))}});
        CHECK(get(get(result, "options")->raw_get<Map>(), "env")
                  ->raw_get<String>()
              == "prod");
    }

    SECTION("Bare dash is treated as a positional argument")
    {
        // Unix convention: `-` is shorthand for stdin/stdout.
        Map pos_entry = {{"name"_s, Value::create("file"s)}};
        Array pos = {Value::create(Value::trusted, std::move(pos_entry))};
        auto result =
            do_parse({"s.frst", "-"},
                     Map{{"positional"_s, Value::create(std::move(pos))}});
        auto positionals = get(result, "positional")->raw_get<Array>();
        REQUIRE(positionals.size() == 1);
        CHECK(positionals.at(0)->raw_get<String>() == "-");
    }
}

TEST_CASE("cli.parse errors")
{
    auto mod = cli_module();
    auto parse = lookup(mod, "parse");

    auto expect_error = [&](std::initializer_list<std::string> argv, Map spec,
                            const std::string& substring) {
        CHECK_THROWS_MATCHES(
            parse->call({Value::create(make_args(argv)),
                         Value::create(Value::trusted, std::move(spec))}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring(substring)));
    };

    SECTION("Unknown long flag")
    {
        expect_error({"s.frst", "--bogus"}, {}, "unknown option '--bogus'");
    }

    SECTION("Unknown short flag")
    {
        expect_error({"s.frst", "-x"}, {}, "unknown option '-x'");
    }

    SECTION("Missing required option")
    {
        Map opt = {{"required"_s, Value::create(Bool{true})}};
        Map options = {
            {"env"_s, Value::create(Value::trusted, std::move(opt))}};
        expect_error({"s.frst"},
                     Map{{"options"_s,
                          Value::create(Value::trusted, std::move(options))}},
                     "missing required option '--env'");
    }

    SECTION("Option missing value")
    {
        Map opt = {};
        Map options = {
            {"env"_s, Value::create(Value::trusted, std::move(opt))}};
        expect_error({"s.frst", "--env"},
                     Map{{"options"_s,
                          Value::create(Value::trusted, std::move(options))}},
                     "requires a value");
    }

    SECTION("Repeated non-repeatable option")
    {
        Map opt = {};
        Map options = {
            {"env"_s, Value::create(Value::trusted, std::move(opt))}};
        expect_error({"s.frst", "--env", "a", "--env", "b"},
                     Map{{"options"_s,
                          Value::create(Value::trusted, std::move(options))}},
                     "cannot be repeated");
    }

    SECTION("Too many positionals")
    {
        Map pos_entry = {{"name"_s, Value::create("one"s)}};
        Array pos = {Value::create(Value::trusted, std::move(pos_entry))};
        expect_error({"s.frst", "a", "b"},
                     Map{{"positional"_s, Value::create(std::move(pos))}},
                     "expected 1 positional");
    }

    SECTION("Too few positionals")
    {
        Map p1 = {{"name"_s, Value::create("one"s)}};
        Map p2 = {{"name"_s, Value::create("two"s)}};
        Array pos = {Value::create(Value::trusted, std::move(p1)),
                     Value::create(Value::trusted, std::move(p2))};
        expect_error({"s.frst", "a"},
                     Map{{"positional"_s, Value::create(std::move(pos))}},
                     "expected 2 positional");
    }

    SECTION("Unexpected positionals when none declared")
    {
        expect_error({"s.frst", "extra"}, {}, "unexpected positional");
    }

    SECTION("Error message uses spec name as prefix")
    {
        CHECK_THROWS_MATCHES(
            parse->call(
                {Value::create(make_args({"s.frst", "--bogus"})),
                 Value::create(Value::trusted,
                               Map{{"name"_s, Value::create("mytool"s)}})}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("mytool:")));
    }

    SECTION("Error message falls back to args[0] when no spec name")
    {
        CHECK_THROWS_MATCHES(
            parse->call({Value::create(make_args({"some-script", "--bogus"})),
                         Value::create(Value::trusted, Map{})}),
            Frost_Recoverable_Error,
            MessageMatches(ContainsSubstring("some-script:")));
    }
}
