#include <catch2/catch_all.hpp>

#include <charconv>
#include <limits>

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

TEST_CASE("Builtin fmt_int")
{
    Symbol_Table table;
    inject_builtins(table);
    auto fmt_int_val = table.lookup("fmt_int");
    REQUIRE(fmt_int_val->is<Function>());
    auto fmt_int = fmt_int_val->get<Function>().value();

    auto expected_fmt = [](Int value, Int base) {
        char buf[66]{};
        const auto [ptr, ec] =
            std::to_chars(std::begin(buf), std::end(buf), value, int(base));
        REQUIRE(ec == std::errc{});
        return String{std::begin(buf), ptr};
    };

    SECTION("Injected")
    {
        CHECK(fmt_int_val->is<Function>());
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(fmt_int->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(fmt_int->call({}),
                          ContainsSubstring("Called with 0"));
        CHECK_THROWS_WITH(fmt_int->call({}),
                          ContainsSubstring("requires at least 2"));

        CHECK_THROWS_WITH(
            fmt_int->call(
                {Value::create(1), Value::create(10), Value::create(2)}),
            ContainsSubstring("too many arguments"));
        CHECK_THROWS_WITH(
            fmt_int->call(
                {Value::create(1), Value::create(10), Value::create(2)}),
            ContainsSubstring("Called with 3"));
        CHECK_THROWS_WITH(
            fmt_int->call(
                {Value::create(1), Value::create(10), Value::create(2)}),
            ContainsSubstring("no more than 2"));
    }

    SECTION("Type errors")
    {
        auto bad_number = Value::create("123"s);
        auto bad_base = Value::create("10"s);

        CHECK_THROWS_WITH(fmt_int->call({bad_number, Value::create(10)}),
                          ContainsSubstring("Int"));
        CHECK_THROWS_WITH(fmt_int->call({bad_number, Value::create(10)}),
                          EndsWith(std::string{bad_number->type_name()}));

        CHECK_THROWS_WITH(fmt_int->call({Value::create(123), bad_base}),
                          ContainsSubstring("Int"));
        CHECK_THROWS_WITH(fmt_int->call({Value::create(123), bad_base}),
                          EndsWith(std::string{bad_base->type_name()}));
    }

    SECTION("Base range checks")
    {
        CHECK_THROWS_WITH(fmt_int->call({Value::create(123), Value::create(1)}),
                          ContainsSubstring("base must be in range [2, 36]"));
        CHECK_THROWS_WITH(
            fmt_int->call({Value::create(123), Value::create(37)}),
            ContainsSubstring("base must be in range [2, 36]"));
    }

    SECTION("Basic formatting")
    {
        auto dec = fmt_int->call({Value::create(123), Value::create(10)});
        auto neg = fmt_int->call({Value::create(-123), Value::create(10)});
        auto hex = fmt_int->call({Value::create(255), Value::create(16)});
        auto bin = fmt_int->call({Value::create(10), Value::create(2)});

        REQUIRE(dec->is<String>());
        REQUIRE(neg->is<String>());
        REQUIRE(hex->is<String>());
        REQUIRE(bin->is<String>());

        CHECK(dec->get<String>().value() == "123");
        CHECK(neg->get<String>().value() == "-123");
        CHECK(hex->get<String>().value() == "ff");
        CHECK(bin->get<String>().value() == "1010");
    }

    SECTION("Int64 extrema")
    {
        constexpr Int min_i = std::numeric_limits<Int>::min();
        constexpr Int max_i = std::numeric_limits<Int>::max();

        auto max_dec = fmt_int->call({Value::create(max_i), Value::create(10)});
        auto min_dec = fmt_int->call({Value::create(min_i), Value::create(10)});
        auto max_bin = fmt_int->call({Value::create(max_i), Value::create(2)});
        auto min_bin = fmt_int->call({Value::create(min_i), Value::create(2)});

        REQUIRE(max_dec->is<String>());
        REQUIRE(min_dec->is<String>());
        REQUIRE(max_bin->is<String>());
        REQUIRE(min_bin->is<String>());

        CHECK(max_dec->get<String>().value() == expected_fmt(max_i, 10));
        CHECK(min_dec->get<String>().value() == expected_fmt(min_i, 10));
        CHECK(max_bin->get<String>().value() == expected_fmt(max_i, 2));
        CHECK(min_bin->get<String>().value() == expected_fmt(min_i, 2));
    }
}

TEST_CASE("Builtin parse_int")
{
    Symbol_Table table;
    inject_builtins(table);
    auto parse_int_val = table.lookup("parse_int");
    REQUIRE(parse_int_val->is<Function>());
    auto parse_int = parse_int_val->get<Function>().value();

    SECTION("Injected")
    {
        CHECK(parse_int_val->is<Function>());
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(parse_int->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(parse_int->call({}),
                          ContainsSubstring("Called with 0"));
        CHECK_THROWS_WITH(parse_int->call({}),
                          ContainsSubstring("requires at least 2"));

        CHECK_THROWS_WITH(
            parse_int->call({Value::create("1"s), Value::create(10),
                             Value::create(2)}),
            ContainsSubstring("too many arguments"));
        CHECK_THROWS_WITH(
            parse_int->call({Value::create("1"s), Value::create(10),
                             Value::create(2)}),
            ContainsSubstring("Called with 3"));
        CHECK_THROWS_WITH(
            parse_int->call({Value::create("1"s), Value::create(10),
                             Value::create(2)}),
            ContainsSubstring("no more than 2"));
    }

    SECTION("Type errors")
    {
        auto bad_number = Value::create(123);
        auto bad_base = Value::create("10"s);

        CHECK_THROWS_WITH(parse_int->call({bad_number, Value::create(10)}),
                          ContainsSubstring("String"));
        CHECK_THROWS_WITH(parse_int->call({bad_number, Value::create(10)}),
                          EndsWith(std::string{bad_number->type_name()}));

        CHECK_THROWS_WITH(parse_int->call({Value::create("123"s), bad_base}),
                          ContainsSubstring("Int"));
        CHECK_THROWS_WITH(parse_int->call({Value::create("123"s), bad_base}),
                          EndsWith(std::string{bad_base->type_name()}));
    }

    SECTION("Base range checks")
    {
        CHECK_THROWS_WITH(
            parse_int->call({Value::create("123"s), Value::create(1)}),
            ContainsSubstring("base must be in range [2, 36]"));
        CHECK_THROWS_WITH(
            parse_int->call({Value::create("123"s), Value::create(37)}),
            ContainsSubstring("base must be in range [2, 36]"));
    }

    SECTION("Basic parsing")
    {
        auto dec = parse_int->call({Value::create("123"s), Value::create(10)});
        auto neg = parse_int->call({Value::create("-123"s), Value::create(10)});
        auto hex_lower =
            parse_int->call({Value::create("ff"s), Value::create(16)});
        auto hex_upper =
            parse_int->call({Value::create("FF"s), Value::create(16)});

        REQUIRE(dec->is<Int>());
        REQUIRE(neg->is<Int>());
        REQUIRE(hex_lower->is<Int>());
        REQUIRE(hex_upper->is<Int>());

        CHECK(dec->get<Int>().value() == 123);
        CHECK(neg->get<Int>().value() == -123);
        CHECK(hex_lower->get<Int>().value() == 255);
        CHECK(hex_upper->get<Int>().value() == 255);
    }

    SECTION("Strict parse guards")
    {
        CHECK_THROWS_WITH(
            parse_int->call({Value::create("+123"s), Value::create(10)}),
            ContainsSubstring("expected numeric string in base 10"));
        CHECK_THROWS_WITH(
            parse_int->call({Value::create("123abc"s), Value::create(10)}),
            ContainsSubstring("expected numeric string in base 10"));
        CHECK_THROWS_WITH(
            parse_int->call({Value::create(" 123"s), Value::create(10)}),
            ContainsSubstring("expected numeric string in base 10"));
        CHECK_THROWS_WITH(
            parse_int->call({Value::create("123 "s), Value::create(10)}),
            ContainsSubstring("expected numeric string in base 10"));
        CHECK_THROWS_WITH(
            parse_int->call({Value::create(""s), Value::create(10)}),
            ContainsSubstring("expected numeric string in base 10"));
        CHECK_THROWS_WITH(
            parse_int->call({Value::create("0x10"s), Value::create(16)}),
            ContainsSubstring("expected numeric string in base 16"));
        CHECK_THROWS_WITH(
            parse_int->call({Value::create("0b10"s), Value::create(2)}),
            ContainsSubstring("expected numeric string in base 2"));
        CHECK_THROWS_WITH(
            parse_int->call({Value::create("2"s), Value::create(2)}),
            ContainsSubstring("expected numeric string in base 2"));
    }

    SECTION("Out of range")
    {
        CHECK_THROWS_WITH(
            parse_int->call({Value::create("999999999999999999999999999999999999"s),
                             Value::create(10)}),
            ContainsSubstring("out of range"));
    }
}
