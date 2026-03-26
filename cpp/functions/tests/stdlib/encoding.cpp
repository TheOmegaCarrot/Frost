#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_exception.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <charconv>
#include <limits>

#include <frost/stdlib.hpp>
#include <frost/value.hpp>

using namespace frst;
using namespace std::literals;
using namespace Catch::Matchers;

namespace
{

Map encoding_module()
{
    Stdlib_Registry_Builder builder;
    register_module_encoding(builder);
    auto registry = std::move(builder).build();
    auto module = registry.lookup_module("std.encoding");
    REQUIRE(module.has_value());
    REQUIRE(module.value()->is<Map>());
    return module.value()->raw_get<Map>();
}

Value_Ptr lookup_value(const Map& mod, const std::string& name)
{
    auto key = Value::create(String{name});
    auto it = mod.find(key);
    REQUIRE(it != mod.end());
    return it->second;
}

Function lookup(const Map& mod, const std::string& name)
{
    auto val = lookup_value(mod, name);
    REQUIRE(val->is<Function>());
    return val->raw_get<Function>();
}

Map lookup_submap(const Map& mod, const std::string& name)
{
    auto val = lookup_value(mod, name);
    REQUIRE(val->is<Map>());
    return val->raw_get<Map>();
}

} // namespace

TEST_CASE("std.encoding")
{
    auto mod = encoding_module();

    SECTION("Registered")
    {
        CHECK(mod.size() == 5);
        lookup_submap(mod, "b64");
        lookup(mod, "fmt_int");
        lookup(mod, "parse_int");
        lookup(mod, "to_bytes");
        lookup(mod, "from_bytes");
    }

    SECTION("b64 submap")
    {
        auto b64 = lookup_submap(mod, "b64");
        CHECK(b64.size() == 4);
        lookup(b64, "encode");
        lookup(b64, "decode");
        lookup(b64, "urlencode");
        lookup(b64, "urldecode");
    }
}

TEST_CASE("std.encoding b64")
{
    auto mod = encoding_module();
    auto b64 = lookup_submap(mod, "b64");

    auto b64_encode = lookup(b64, "encode");
    auto b64_decode = lookup(b64, "decode");
    auto b64_urlencode = lookup(b64, "urlencode");
    auto b64_urldecode = lookup(b64, "urldecode");

    SECTION("RFC 4648 base64 known vectors")
    {
        struct Case
        {
            std::string input;
            std::string encoded;
        };

        const std::vector<Case> cases{
            {""s, ""s},        {"f"s, "Zg=="s},         {"fo"s, "Zm8="s},
            {"foo"s, "Zm9v"s}, {"hello"s, "aGVsbG8="s},
        };

        for (const auto& c : cases)
        {
            auto enc = b64_encode->call({Value::create(String{c.input})});
            REQUIRE(enc->is<String>());
            CHECK(enc->raw_get<String>() == c.encoded);

            auto dec = b64_decode->call({Value::create(String{c.encoded})});
            REQUIRE(dec->is<String>());
            CHECK(dec->raw_get<String>() == c.input);
        }
    }

    SECTION("URL-safe base64 encodes with '-' and '_' and requires padding")
    {
        std::string byte_ff{static_cast<char>(0xff)};
        std::string byte_fa{static_cast<char>(0xfa)};

        auto std_ff = b64_encode->call({Value::create(String{byte_ff})});
        REQUIRE(std_ff->is<String>());
        CHECK(std_ff->raw_get<String>() == "/w=="s);

        auto url_ff = b64_urlencode->call({Value::create(String{byte_ff})});
        REQUIRE(url_ff->is<String>());
        CHECK(url_ff->raw_get<String>() == "_w=="s);

        auto url_fa = b64_urlencode->call({Value::create(String{byte_fa})});
        REQUIRE(url_fa->is<String>());
        CHECK(url_fa->raw_get<String>() == "-g=="s);

        auto dec_ff = b64_urldecode->call({Value::create(String{"_w=="s})});
        REQUIRE(dec_ff->is<String>());
        const auto& out_ff = dec_ff->raw_get<String>();
        REQUIRE(out_ff.size() == 1);
        CHECK(static_cast<unsigned char>(out_ff.at(0)) == 0xff);
    }

    SECTION("Invalid input throws recoverable errors")
    {
        CHECK_THROWS_AS(b64_decode->call({Value::create(String{"%%%"s})}),
                        Frost_Recoverable_Error);
        CHECK_THROWS_AS(b64_urldecode->call({Value::create(String{"Zg"s})}),
                        Frost_Recoverable_Error);
        CHECK_THROWS_AS(b64_urldecode->call({Value::create(String{"Zg*="s})}),
                        Frost_Recoverable_Error);
    }

    SECTION("Arity")
    {
        CHECK_THROWS_MATCHES(
            b64_encode->call({}), Frost_User_Error,
            MessageMatches(ContainsSubstring("insufficient arguments")
                           && ContainsSubstring("requires at least 1")));
        CHECK_THROWS_MATCHES(
            b64_decode->call({}), Frost_User_Error,
            MessageMatches(ContainsSubstring("insufficient arguments")
                           && ContainsSubstring("requires at least 1")));
        CHECK_THROWS_MATCHES(
            b64_urlencode->call({}), Frost_User_Error,
            MessageMatches(ContainsSubstring("insufficient arguments")
                           && ContainsSubstring("requires at least 1")));
        CHECK_THROWS_MATCHES(
            b64_urldecode->call({}), Frost_User_Error,
            MessageMatches(ContainsSubstring("insufficient arguments")
                           && ContainsSubstring("requires at least 1")));
    }

    SECTION("Type errors")
    {
        CHECK_THROWS_MATCHES(
            b64_encode->call({Value::create(1)}), Frost_User_Error,
            MessageMatches(ContainsSubstring("encoding.b64.encode")
                           && ContainsSubstring("String")));
        CHECK_THROWS_MATCHES(
            b64_decode->call({Value::create(1)}), Frost_User_Error,
            MessageMatches(ContainsSubstring("encoding.b64.decode")
                           && ContainsSubstring("String")));
    }
}

TEST_CASE("std.encoding fmt_int")
{
    auto mod = encoding_module();
    auto fmt_int = lookup(mod, "fmt_int");

    auto expected_fmt = [](Int value, Int base) {
        char buf[66]{};
        const auto [ptr, ec] =
            std::to_chars(std::begin(buf), std::end(buf), value, int(base));
        REQUIRE(ec == std::errc{});
        return String{std::begin(buf), ptr};
    };

    SECTION("Arity")
    {
        CHECK_THROWS_MATCHES(
            fmt_int->call({}), Frost_User_Error,
            MessageMatches(ContainsSubstring("insufficient arguments")
                           && ContainsSubstring("requires at least 2")));
        CHECK_THROWS_MATCHES(
            fmt_int->call(
                {Value::create(1), Value::create(10), Value::create(2)}),
            Frost_User_Error,
            MessageMatches(ContainsSubstring("too many arguments")
                           && ContainsSubstring("no more than 2")));
    }

    SECTION("Type errors")
    {
        auto bad_number = Value::create("123"s);
        auto bad_base = Value::create("10"s);

        CHECK_THROWS_MATCHES(
            fmt_int->call({bad_number, Value::create(10)}), Frost_User_Error,
            MessageMatches(ContainsSubstring("encoding.fmt_int")
                           && ContainsSubstring("Int")
                           && ContainsSubstring("number")));

        CHECK_THROWS_MATCHES(
            fmt_int->call({Value::create(123), bad_base}), Frost_User_Error,
            MessageMatches(ContainsSubstring("Int")
                           && ContainsSubstring("base")));
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

TEST_CASE("std.encoding parse_int")
{
    auto mod = encoding_module();
    auto parse_int = lookup(mod, "parse_int");

    SECTION("Arity")
    {
        CHECK_THROWS_MATCHES(
            parse_int->call({}), Frost_User_Error,
            MessageMatches(ContainsSubstring("insufficient arguments")
                           && ContainsSubstring("requires at least 2")));
        CHECK_THROWS_MATCHES(
            parse_int->call(
                {Value::create("1"s), Value::create(10), Value::create(2)}),
            Frost_User_Error,
            MessageMatches(ContainsSubstring("too many arguments")
                           && ContainsSubstring("no more than 2")));
    }

    SECTION("Type errors")
    {
        auto bad_number = Value::create(123);
        auto bad_base = Value::create("10"s);

        CHECK_THROWS_MATCHES(
            parse_int->call({bad_number, Value::create(10)}), Frost_User_Error,
            MessageMatches(ContainsSubstring("encoding.parse_int")
                           && ContainsSubstring("String")));

        CHECK_THROWS_MATCHES(
            parse_int->call({Value::create("123"s), bad_base}), Frost_User_Error,
            MessageMatches(ContainsSubstring("Int")
                           && ContainsSubstring("base")));
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
            parse_int->call(
                {Value::create("999999999999999999999999999999999999"s),
                 Value::create(10)}),
            ContainsSubstring("out of range"));
    }
}

TEST_CASE("std.encoding to_bytes/from_bytes")
{
    auto mod = encoding_module();
    auto to_bytes = lookup(mod, "to_bytes");
    auto from_bytes = lookup(mod, "from_bytes");

    SECTION("Arity")
    {
        CHECK_THROWS_MATCHES(
            to_bytes->call({}), Frost_User_Error,
            MessageMatches(ContainsSubstring("insufficient arguments")
                           && ContainsSubstring("requires at least 1")));
        CHECK_THROWS_MATCHES(
            to_bytes->call({Value::create("a"s), Value::create("b"s)}),
            Frost_User_Error,
            MessageMatches(ContainsSubstring("too many arguments")
                           && ContainsSubstring("no more than 1")));

        CHECK_THROWS_MATCHES(
            from_bytes->call({}), Frost_User_Error,
            MessageMatches(ContainsSubstring("insufficient arguments")
                           && ContainsSubstring("requires at least 1")));
        CHECK_THROWS_MATCHES(
            from_bytes->call(
                {Value::create(Array{}), Value::create(Array{})}),
            Frost_User_Error,
            MessageMatches(ContainsSubstring("too many arguments")
                           && ContainsSubstring("no more than 1")));
    }

    SECTION("Type errors")
    {
        auto bad_string_arg = Value::create(123_f);
        CHECK_THROWS_MATCHES(
            to_bytes->call({bad_string_arg}), Frost_User_Error,
            MessageMatches(ContainsSubstring("encoding.to_bytes")
                           && ContainsSubstring("String")));

        auto bad_array_arg = Value::create("abc"s);
        CHECK_THROWS_MATCHES(
            from_bytes->call({bad_array_arg}), Frost_User_Error,
            MessageMatches(ContainsSubstring("encoding.from_bytes")
                           && ContainsSubstring("Array")));
    }

    SECTION("to_bytes returns unsigned byte values")
    {
        String input;
        input.push_back('\0');
        input.push_back('\x01');
        input.push_back('\x7f');
        input.push_back(static_cast<char>(0x80));
        input.push_back(static_cast<char>(0xff));

        auto out = to_bytes->call({Value::create(std::move(input))});
        REQUIRE(out->is<Array>());
        const auto& arr = out->raw_get<Array>();

        REQUIRE(arr.size() == 5);
        CHECK(arr[0]->get<Int>().value() == 0);
        CHECK(arr[1]->get<Int>().value() == 1);
        CHECK(arr[2]->get<Int>().value() == 127);
        CHECK(arr[3]->get<Int>().value() == 128);
        CHECK(arr[4]->get<Int>().value() == 255);
    }

    SECTION("from_bytes accepts [0, 255] and preserves bytes")
    {
        Array input{
            Value::create(Int{0}),   Value::create(Int{1}),
            Value::create(Int{127}), Value::create(Int{128}),
            Value::create(Int{255}),
        };

        auto out = from_bytes->call({Value::create(std::move(input))});
        REQUIRE(out->is<String>());
        const auto& str = out->raw_get<String>();

        REQUIRE(str.size() == 5);
        CHECK(static_cast<unsigned char>(str[0]) == 0);
        CHECK(static_cast<unsigned char>(str[1]) == 1);
        CHECK(static_cast<unsigned char>(str[2]) == 127);
        CHECK(static_cast<unsigned char>(str[3]) == 128);
        CHECK(static_cast<unsigned char>(str[4]) == 255);
    }

    SECTION("Round-trip preserves embedded NUL and high bytes")
    {
        String input;
        input.push_back('x');
        input.push_back('\0');
        input.push_back('y');
        input.push_back(static_cast<char>(0x80));
        input.push_back(static_cast<char>(0xff));

        auto bytes = to_bytes->call({Value::create(String{input})});
        auto round_trip = from_bytes->call({bytes});

        REQUIRE(round_trip->is<String>());
        CHECK(round_trip->raw_get<String>() == input);
    }

    SECTION("from_bytes rejects non-Int elements")
    {
        CHECK_THROWS_WITH(from_bytes->call({Value::create(
                              Array{Value::create(1.5), Value::create(2)})}),
                          ContainsSubstring("expected Array of Int"));
        CHECK_THROWS_WITH(from_bytes->call({Value::create(
                              Array{Value::create(true), Value::create(2)})}),
                          ContainsSubstring("expected Array of Int"));
    }

    SECTION("from_bytes rejects out-of-range Int elements")
    {
        CHECK_THROWS_WITH(from_bytes->call(
                              {Value::create(Array{Value::create(Int{-1})})}),
                          ContainsSubstring("range [0, 255]"));
        CHECK_THROWS_WITH(from_bytes->call(
                              {Value::create(Array{Value::create(Int{256})})}),
                          ContainsSubstring("range [0, 255]"));
    }

    SECTION("Empty input")
    {
        auto bytes = to_bytes->call({Value::create(""s)});
        REQUIRE(bytes->is<Array>());
        CHECK(bytes->raw_get<Array>().empty());

        auto str = from_bytes->call({Value::create(Array{})});
        REQUIRE(str->is<String>());
        CHECK(str->raw_get<String>().empty());
    }
}
