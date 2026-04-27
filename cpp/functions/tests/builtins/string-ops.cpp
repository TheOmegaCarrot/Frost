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
                          ContainsSubstring("delimiter"));
        CHECK_THROWS_WITH(split->call({good, bad_second}),
                          EndsWith(std::string{bad_second->type_name()}));
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

    SECTION("Multichar split")
    {
        auto res =
            split->call({Value::create("a,*b,*c"s), Value::create(",*"s)});
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
                CHECK_THROWS_WITH(fn->call({bad}), ContainsSubstring("String"));
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

        auto upper = get_fn("to_upper")->call({input})->get<String>().value();
        auto lower = get_fn("to_lower")->call({input})->get<String>().value();

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

TEST_CASE("Builtin join")
{
    Symbol_Table table;
    inject_builtins(table);
    auto join_val = table.lookup("join");
    REQUIRE(join_val->is<Function>());
    auto join = join_val->get<Function>().value();

    SECTION("Injected")
    {
        CHECK(join_val->is<Function>());
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(join->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(join->call({Value::create(Array{})}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(
            join->call({Value::create(Array{}), Value::create(","s),
                        Value::create("extra"s)}),
            ContainsSubstring("too many arguments"));
    }

    SECTION("Type errors")
    {
        auto bad_arr = Value::create("not_an_array"s);
        auto bad_sep = Value::create(42_f);
        auto good_arr = Value::create(Array{});
        auto good_sep = Value::create(","s);

        CHECK_THROWS_WITH(join->call({bad_arr, good_sep}),
                          ContainsSubstring("join"));
        CHECK_THROWS_WITH(join->call({bad_arr, good_sep}),
                          ContainsSubstring("Array"));
        CHECK_THROWS_WITH(join->call({bad_arr, good_sep}),
                          EndsWith(std::string{bad_arr->type_name()}));

        CHECK_THROWS_WITH(join->call({good_arr, bad_sep}),
                          ContainsSubstring("join"));
        CHECK_THROWS_WITH(join->call({good_arr, bad_sep}),
                          ContainsSubstring("String"));
        CHECK_THROWS_WITH(join->call({good_arr, bad_sep}),
                          EndsWith(std::string{bad_sep->type_name()}));
    }

    SECTION("Non-String array elements are rejected")
    {
        auto arr = Value::create(Array{Value::create("a"s), Value::create(42_f),
                                       Value::create("b"s)});
        CHECK_THROWS_WITH(join->call({arr, Value::create(","s)}),
                          ContainsSubstring("join"));
        CHECK_THROWS_WITH(join->call({arr, Value::create(","s)}),
                          ContainsSubstring("Array of Strings"));
        CHECK_THROWS_WITH(join->call({arr, Value::create(","s)}),
                          ContainsSubstring("Int"));
    }

    SECTION("Basic behavior")
    {
        auto arr =
            Value::create(Array{Value::create("foo"s), Value::create("bar"s)});
        auto res = join->call({arr, Value::create(" "s)});
        REQUIRE(res->is<String>());
        CHECK(res->raw_get<String>() == "foo bar");
    }

    SECTION("Single-element array returns that element")
    {
        auto arr = Value::create(Array{Value::create("only"s)});
        auto res = join->call({arr, Value::create(","s)});
        REQUIRE(res->is<String>());
        CHECK(res->raw_get<String>() == "only");
    }

    SECTION("Empty array returns empty string")
    {
        auto res = join->call({Value::create(Array{}), Value::create(","s)});
        REQUIRE(res->is<String>());
        CHECK(res->raw_get<String>().empty());
    }

    SECTION("Empty separator concatenates directly")
    {
        auto arr = Value::create(Array{Value::create("a"s), Value::create("b"s),
                                       Value::create("c"s)});
        auto res = join->call({arr, Value::create(""s)});
        REQUIRE(res->is<String>());
        CHECK(res->raw_get<String>() == "abc");
    }

    SECTION("Multi-char separator")
    {
        auto arr = Value::create(Array{Value::create("a"s), Value::create("b"s),
                                       Value::create("c"s)});
        auto res = join->call({arr, Value::create(", "s)});
        REQUIRE(res->is<String>());
        CHECK(res->raw_get<String>() == "a, b, c");
    }

    SECTION("Empty strings in array are included")
    {
        auto arr = Value::create(Array{Value::create("a"s), Value::create(""s),
                                       Value::create("b"s)});
        auto res = join->call({arr, Value::create("-"s)});
        REQUIRE(res->is<String>());
        CHECK(res->raw_get<String>() == "a--b");
    }
}

TEST_CASE("Builtin trim/trim_left/trim_right")
{
    Symbol_Table table;
    inject_builtins(table);

    const std::vector<std::string> names{"trim", "trim_left", "trim_right"};

    auto get_fn = [&](std::string_view name) {
        auto val = table.lookup(std::string{name});
        REQUIRE(val->is<Function>());
        return val->get<Function>().value();
    };

    SECTION("Injected")
    {
        for (const auto& name : names)
            CHECK(table.lookup(name)->is<Function>());
    }

    SECTION("Arity and type errors")
    {
        for (const auto& name : names)
        {
            DYNAMIC_SECTION(name)
            {
                auto fn = get_fn(name);
                CHECK_THROWS_WITH(fn->call({}),
                                  ContainsSubstring("insufficient arguments"));
                CHECK_THROWS_WITH(
                    fn->call({Value::create("a"s), Value::create("b"s)}),
                    ContainsSubstring("too many arguments"));
                CHECK_THROWS_WITH(fn->call({Value::create(1)}),
                                  ContainsSubstring("String"));
            }
        }
    }

    SECTION("No whitespace is unchanged")
    {
        auto val = Value::create("hello"s);
        CHECK(get_fn("trim")->call({val})->raw_get<String>() == "hello");
        CHECK(get_fn("trim_left")->call({val})->raw_get<String>() == "hello");
        CHECK(get_fn("trim_right")->call({val})->raw_get<String>() == "hello");
    }

    SECTION("Leading and trailing spaces")
    {
        auto val = Value::create("  hello  "s);
        CHECK(get_fn("trim")->call({val})->raw_get<String>() == "hello");
        CHECK(get_fn("trim_left")->call({val})->raw_get<String>() == "hello  ");
        CHECK(get_fn("trim_right")->call({val})->raw_get<String>()
              == "  hello");
    }

    SECTION("Only leading whitespace")
    {
        auto val = Value::create("   hello"s);
        CHECK(get_fn("trim")->call({val})->raw_get<String>() == "hello");
        CHECK(get_fn("trim_left")->call({val})->raw_get<String>() == "hello");
        CHECK(get_fn("trim_right")->call({val})->raw_get<String>()
              == "   hello");
    }

    SECTION("Only trailing whitespace")
    {
        auto val = Value::create("hello   "s);
        CHECK(get_fn("trim")->call({val})->raw_get<String>() == "hello");
        CHECK(get_fn("trim_left")->call({val})->raw_get<String>()
              == "hello   ");
        CHECK(get_fn("trim_right")->call({val})->raw_get<String>() == "hello");
    }

    SECTION("Tabs and mixed whitespace")
    {
        auto val = Value::create("\t hello \t"s);
        CHECK(get_fn("trim")->call({val})->raw_get<String>() == "hello");
        CHECK(get_fn("trim_left")->call({val})->raw_get<String>()
              == "hello \t");
        CHECK(get_fn("trim_right")->call({val})->raw_get<String>()
              == "\t hello");
    }

    SECTION("Newlines are treated as whitespace")
    {
        auto val = Value::create("\nhello\n"s);
        CHECK(get_fn("trim")->call({val})->raw_get<String>() == "hello");
        CHECK(get_fn("trim_left")->call({val})->raw_get<String>() == "hello\n");
        CHECK(get_fn("trim_right")->call({val})->raw_get<String>()
              == "\nhello");
    }

    SECTION("Internal whitespace is preserved")
    {
        auto val = Value::create("  hello   world  "s);
        CHECK(get_fn("trim")->call({val})->raw_get<String>()
              == "hello   world");
    }

    SECTION("All whitespace returns empty string")
    {
        auto val = Value::create("   "s);
        for (const auto& name : names)
            CHECK(get_fn(name)->call({val})->raw_get<String>().empty());
    }

    SECTION("Empty string returns empty string")
    {
        auto val = Value::create(""s);
        for (const auto& name : names)
            CHECK(get_fn(name)->call({val})->raw_get<String>().empty());
    }
}

TEST_CASE("Builtin replace")
{
    Symbol_Table table;
    inject_builtins(table);
    auto fn_val = table.lookup("replace");
    REQUIRE(fn_val->is<Function>());
    auto fn = fn_val->get<Function>().value();

    SECTION("Injected")
    {
        CHECK(fn_val->is<Function>());
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(fn->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(fn->call({Value::create("a"s), Value::create("b"s)}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(fn->call({Value::create("a"s), Value::create("b"s),
                                    Value::create("c"s), Value::create("d"s)}),
                          ContainsSubstring("too many arguments"));
    }

    SECTION("Type errors")
    {
        auto bad = Value::create(1);
        auto good = Value::create("x"s);
        CHECK_THROWS_WITH(fn->call({bad, good, good}),
                          ContainsSubstring("replace")
                              && ContainsSubstring("String"));
        CHECK_THROWS_WITH(fn->call({good, bad, good}),
                          ContainsSubstring("replace")
                              && ContainsSubstring("find"));
        CHECK_THROWS_WITH(fn->call({good, good, bad}),
                          ContainsSubstring("replace")
                              && ContainsSubstring("replacement"));
    }

    SECTION("Basic replacement")
    {
        auto r = fn->call({Value::create("hello world"s),
                           Value::create("world"s), Value::create("frost"s)});
        REQUIRE(r->is<String>());
        CHECK(r->raw_get<String>() == "hello frost");
    }

    SECTION("Replaces all occurrences")
    {
        auto r = fn->call({Value::create("aXbXcX"s), Value::create("X"s),
                           Value::create("-"s)});
        REQUIRE(r->is<String>());
        CHECK(r->raw_get<String>() == "a-b-c-");
    }

    SECTION("Find not present returns original")
    {
        auto r = fn->call({Value::create("hello"s), Value::create("xyz"s),
                           Value::create("!"s)});
        REQUIRE(r->is<String>());
        CHECK(r->raw_get<String>() == "hello");
    }

    SECTION("Empty replacement deletes matches")
    {
        auto r = fn->call(
            {Value::create("hello"s), Value::create("l"s), Value::create(""s)});
        REQUIRE(r->is<String>());
        CHECK(r->raw_get<String>() == "heo");
    }

    SECTION("Multichar find")
    {
        auto r = fn->call({Value::create("abcXYZabc"s), Value::create("XYZ"s),
                           Value::create("---"s)});
        REQUIRE(r->is<String>());
        CHECK(r->raw_get<String>() == "abc---abc");
    }

    SECTION("Non-overlapping: only consumed chars are replaced")
    {
        // "aaaa" with find="aa", replacement="b": left-to-right non-overlapping
        // matches at [0,1] and [2,3] -> "bb"
        auto r = fn->call({Value::create("aaaa"s), Value::create("aa"s),
                           Value::create("b"s)});
        REQUIRE(r->is<String>());
        CHECK(r->raw_get<String>() == "bb");
    }
}

TEST_CASE("Builtin lines")
{
    Symbol_Table table;
    inject_builtins(table);
    auto fn_val = table.lookup("lines");
    REQUIRE(fn_val->is<Function>());
    auto fn = fn_val->get<Function>().value();

    auto get_strings = [](const Value_Ptr& v) {
        REQUIRE(v->is<Array>());
        std::vector<String> result;
        for (const auto& elem : v->raw_get<Array>())
        {
            REQUIRE(elem->is<String>());
            result.push_back(elem->raw_get<String>());
        }
        return result;
    };

    SECTION("Injected")
    {
        CHECK(fn_val->is<Function>());
    }

    SECTION("Arity and type errors")
    {
        CHECK_THROWS_WITH(fn->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(fn->call({Value::create("a"s), Value::create("b"s)}),
                          ContainsSubstring("too many arguments"));
        CHECK_THROWS_WITH(fn->call({Value::create(1)}),
                          ContainsSubstring("String"));
    }

    SECTION("Empty string returns empty array")
    {
        auto r = fn->call({Value::create(""s)});
        REQUIRE(r->is<Array>());
        CHECK(r->raw_get<Array>().empty());
    }

    SECTION("Single line with no newline")
    {
        auto strs = get_strings(fn->call({Value::create("hello"s)}));
        REQUIRE(strs.size() == 1);
        CHECK(strs[0] == "hello");
    }

    SECTION("Multiple lines")
    {
        auto strs = get_strings(fn->call({Value::create("a\nb\nc"s)}));
        REQUIRE(strs.size() == 3);
        CHECK(strs[0] == "a");
        CHECK(strs[1] == "b");
        CHECK(strs[2] == "c");
    }

    SECTION("Trailing newline produces empty final element")
    {
        auto strs = get_strings(fn->call({Value::create("a\nb\n"s)}));
        REQUIRE(strs.size() == 3);
        CHECK(strs[0] == "a");
        CHECK(strs[1] == "b");
        CHECK(strs[2] == "");
    }

    SECTION("Consecutive newlines produce empty elements")
    {
        auto strs = get_strings(fn->call({Value::create("a\n\nb"s)}));
        REQUIRE(strs.size() == 3);
        CHECK(strs[0] == "a");
        CHECK(strs[1] == "");
        CHECK(strs[2] == "b");
    }
}

TEST_CASE("Builtin contains: suggests includes for arrays")
{
    Symbol_Table table;
    inject_builtins(table);
    auto contains = table.lookup("contains")->get<Function>().value();

    SECTION("String usage still works")
    {
        auto result = contains->call(
            {Value::create("hello world"s), Value::create("world"s)});
        CHECK(result->raw_get<Bool>() == true);
    }

    SECTION("Array as first arg suggests includes")
    {
        CHECK_THROWS_WITH(
            contains->call(
                {Value::create(Array{Value::create(1_f)}), Value::create(1_f)}),
            ContainsSubstring("includes"));
    }
}
