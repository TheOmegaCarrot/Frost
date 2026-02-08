#include <catch2/catch_all.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

using namespace frst;
using namespace std::literals;

namespace
{
Function lookup_fn(Symbol_Table& table, const std::string& name)
{
    auto val = table.lookup(name);
    REQUIRE(val->is<Function>());
    return val->get<Function>().value();
}
}

TEST_CASE("Builtin base64")
{
    Symbol_Table table;
    inject_builtins(table);

    auto b64_encode = lookup_fn(table, "b64_encode");
    auto b64_decode = lookup_fn(table, "b64_decode");
    auto b64_urlencode = lookup_fn(table, "b64_urlencode");
    auto b64_urldecode = lookup_fn(table, "b64_urldecode");

    SECTION("RFC 4648 base64 known vectors")
    {
        struct Case
        {
            std::string input;
            std::string encoded;
        };

        const std::vector<Case> cases{
            {""s, ""s},
            {"f"s, "Zg=="s},
            {"fo"s, "Zm8="s},
            {"foo"s, "Zm9v"s},
            {"hello"s, "aGVsbG8="s},
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
}
