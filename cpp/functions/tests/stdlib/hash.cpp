#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/stdlib.hpp>
#include <frost/value.hpp>

using namespace frst;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;

namespace
{

Map hash_module()
{
    Stdlib_Registry_Builder builder;
    register_module_hash(builder);
    auto registry = std::move(builder).build();
    auto module = registry.lookup_module("std.hash");
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

TEST_CASE("stdlib::hash")
{
    auto mod = hash_module();

    SECTION("All algorithms are registered")
    {
        const std::string_view expected[] = {
            "md5",        "sha1",       "sha224",     "sha256",
            "sha384",     "sha512",     "sha3_224",   "sha3_256",
            "sha3_384",   "sha3_512",   "blake2s256", "blake2b512",
            "ripemd160",  "sha512_224", "sha512_256", "sm3",
        };

        for (auto name : expected)
        {
            auto key = Value::create(String{std::string{name}});
            auto it = mod.find(key);
            REQUIRE(it != mod.end());
            CHECK(it->second->is<Function>());
        }
    }

    SECTION("Arity: too few arguments")
    {
        auto sha256 = lookup(mod, "sha256");
        CHECK_THROWS_WITH(sha256->call({}),
                          ContainsSubstring("insufficient arguments"));
    }

    SECTION("Arity: too many arguments")
    {
        auto sha256 = lookup(mod, "sha256");
        CHECK_THROWS_WITH(
            sha256->call({Value::create("a"s), Value::create("b"s)}),
            ContainsSubstring("too many arguments"));
    }

    SECTION("Type constraint: requires String")
    {
        auto sha256 = lookup(mod, "sha256");
        CHECK_THROWS_WITH(sha256->call({Value::create(42_f)}),
                          ContainsSubstring("String"));
        CHECK_THROWS_WITH(sha256->call({Value::null()}),
                          ContainsSubstring("String"));
        CHECK_THROWS_WITH(sha256->call({Value::create(true)}),
                          ContainsSubstring("String"));
    }

    SECTION("Reference outputs: empty string")
    {
        auto sha256 = lookup(mod, "sha256");
        auto result = sha256->call({Value::create(""s)});
        CHECK(result->raw_get<String>()
              == "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    }

    SECTION("Reference outputs: \"hello\"")
    {
        auto input = Value::create("hello"s);

        struct Case
        {
            std::string name;
            std::string expected;
        };

        const Case cases[] = {
            {"md5", "5d41402abc4b2a76b9719d911017c592"},
            {"sha1", "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"},
            {"sha224", "ea09ae9cc6768c50fcee903ed054556e5bfc8347907f12598aa24193"},
            {"sha256", "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"},
            {"sha384", "59e1748777448c69de6b800d7a33bbfb9ff1b463e44354c3553bcdb9c666fa90125a3c79f90397bdf5f6a13de828684f"},
            {"sha512", "9b71d224bd62f3785d96d46ad3ea3d73319bfbc2890caadae2dff72519673ca72323c3d99ba5c11d7c7acc6e14b8c5da0c4663475c2e5c3adef46f73bcdec043"},
            {"sha3_224", "b87f88c72702fff1748e58b87e9141a42c0dbedc29a78cb0d4a5cd81"},
            {"sha3_256", "3338be694f50c5f338814986cdf0686453a888b84f424d792af4b9202398f392"},
            {"sha3_384", "720aea11019ef06440fbf05d87aa24680a2153df3907b23631e7177ce620fa1330ff07c0fddee54699a4c3ee0ee9d887"},
            {"sha3_512", "75d527c368f2efe848ecf6b073a36767800805e9eef2b1857d5f984f036eb6df891d75f72d9b154518c1cd58835286d1da9a38deba3de98b5a53e5ed78a84976"},
            {"blake2s256", "19213bacc58dee6dbde3ceb9a47cbb330b3d86f8cca8997eb00be456f140ca25"},
            {"blake2b512", "e4cfa39a3d37be31c59609e807970799caa68a19bfaa15135f165085e01d41a65ba1e1b146aeb6bd0092b49eac214c103ccfa3a365954bbbe52f74a2b3620c94"},
            {"ripemd160", "108f07b8382412612c048d07d13f814118445acd"},
            {"sha512_224", "fe8509ed1fb7dcefc27e6ac1a80eddbec4cb3d2c6fe565244374061c"},
            {"sha512_256", "e30d87cfa2a75db545eac4d61baf970366a8357c7f72fa95b52d0accb698f13a"},
            {"sm3", "becbbfaae6548b8bf0cfcad5a27183cd1be6093b1cceccc303d9c61d0a645268"},
        };

        for (const auto& c : cases)
        {
            auto fn = lookup(mod, c.name);
            auto result = fn->call({input});
            REQUIRE(result->is<String>());
            CHECK(result->raw_get<String>() == c.expected);
        }
    }

    SECTION("Deterministic: same input produces same output")
    {
        auto sha256 = lookup(mod, "sha256");
        auto input = Value::create("test"s);
        auto a = sha256->call({input});
        auto b = sha256->call({input});
        CHECK(a->raw_get<String>() == b->raw_get<String>());
    }

    SECTION("Different inputs produce different outputs")
    {
        auto sha256 = lookup(mod, "sha256");
        auto a = sha256->call({Value::create("hello"s)});
        auto b = sha256->call({Value::create("world"s)});
        CHECK(a->raw_get<String>() != b->raw_get<String>());
    }
}
