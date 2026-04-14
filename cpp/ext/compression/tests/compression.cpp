#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/extensions-common.hpp>
#include <frost/value.hpp>

using namespace frst;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;

namespace frst
{
void register_module_compression(Stdlib_Registry_Builder&);
}

namespace
{

Map compression_module()
{
    Stdlib_Registry_Builder builder;
    register_module_compression(builder);
    auto registry = std::move(builder).build();
    auto module = registry.lookup_module("ext.compression");
    REQUIRE(module.has_value());
    REQUIRE(module.value()->is<Map>());
    return module.value()->raw_get<Map>();
}

Map lookup_algo(const Map& mod, const std::string& name)
{
    auto key = Value::create(String{name});
    auto it = mod.find(key);
    REQUIRE(it != mod.end());
    REQUIRE(it->second->is<Map>());
    return it->second->raw_get<Map>();
}

Function lookup_fn(const Map& algo, const std::string& name)
{
    auto key = Value::create(String{name});
    auto it = algo.find(key);
    REQUIRE(it != algo.end());
    REQUIRE(it->second->is<Function>());
    return it->second->raw_get<Function>();
}

Value_Ptr call1(const Function& fn, Value_Ptr arg)
{
    std::vector<Value_Ptr> args{std::move(arg)};
    return fn->call(args);
}

Value_Ptr call2(const Function& fn, Value_Ptr a, Value_Ptr b)
{
    std::vector<Value_Ptr> args{std::move(a), std::move(b)};
    return fn->call(args);
}

constexpr std::string_view all_algos[] = {"brotli", "deflate", "gzip", "zlib"};
constexpr std::string_view zlib_algos[] = {"deflate", "gzip", "zlib"};

} // namespace

// =============================================================================
// Module structure
// =============================================================================

TEST_CASE("ext::compression: all algorithms are registered")
{
    auto mod = compression_module();

    for (auto name : all_algos)
    {
        auto algo = lookup_algo(mod, std::string{name});
        auto compress_key = Value::create("compress"s);
        auto decompress_key = Value::create("decompress"s);
        CHECK(algo.find(compress_key) != algo.end());
        CHECK(algo.find(decompress_key) != algo.end());
    }
}

// =============================================================================
// Arity and type checks (shared across all algorithms)
// =============================================================================

TEST_CASE("ext::compression: arity checks")
{
    auto mod = compression_module();

    for (auto name : all_algos)
    {
        DYNAMIC_SECTION(name)
        {
            auto algo = lookup_algo(mod, std::string{name});
            auto compress = lookup_fn(algo, "compress");
            auto decompress = lookup_fn(algo, "decompress");

            CHECK_THROWS_WITH(compress->call({}),
                              ContainsSubstring("insufficient arguments"));
            CHECK_THROWS_WITH(decompress->call({}),
                              ContainsSubstring("insufficient arguments"));
        }
    }
}

TEST_CASE("ext::compression: type checks")
{
    auto mod = compression_module();

    for (auto name : all_algos)
    {
        DYNAMIC_SECTION(name)
        {
            auto algo = lookup_algo(mod, std::string{name});
            auto compress = lookup_fn(algo, "compress");
            auto decompress = lookup_fn(algo, "decompress");

            CHECK_THROWS(call1(compress, Value::create(42_f)));
            CHECK_THROWS(call1(compress, Value::null()));
            CHECK_THROWS(call1(compress, Value::create(true)));
            CHECK_THROWS(call1(compress, Value::create(Array{})));

            CHECK_THROWS(call1(decompress, Value::create(42_f)));
            CHECK_THROWS(call1(decompress, Value::null()));
        }
    }
}

TEST_CASE("ext::compression: level/quality must be Int")
{
    auto mod = compression_module();

    for (auto name : all_algos)
    {
        DYNAMIC_SECTION(name)
        {
            auto compress =
                lookup_fn(lookup_algo(mod, std::string{name}), "compress");
            CHECK_THROWS(call2(compress, Value::create(""s),
                               Value::create("fast"s)));
        }
    }
}

TEST_CASE("ext::compression: zlib level out of range")
{
    auto mod = compression_module();

    for (auto name : zlib_algos)
    {
        DYNAMIC_SECTION(name)
        {
            auto compress =
                lookup_fn(lookup_algo(mod, std::string{name}), "compress");
            CHECK_THROWS_WITH(
                call2(compress, Value::create(""s), Value::create(10_f)),
                ContainsSubstring("level must be between -1 and 9"));
            CHECK_THROWS_WITH(
                call2(compress, Value::create(""s), Value::create(-2_f)),
                ContainsSubstring("level must be between -1 and 9"));
        }
    }
}

TEST_CASE("ext::compression: brotli quality out of range")
{
    auto mod = compression_module();
    auto compress = lookup_fn(lookup_algo(mod, "brotli"), "compress");

    CHECK_THROWS_WITH(
        call2(compress, Value::create(""s), Value::create(12_f)),
        ContainsSubstring("quality must be between"));
    CHECK_THROWS_WITH(
        call2(compress, Value::create(""s), Value::create(-1_f)),
        ContainsSubstring("quality must be between"));
}

// =============================================================================
// Round-trip (per algorithm)
// =============================================================================

TEST_CASE("ext::compression: round-trip")
{
    auto mod = compression_module();

    for (auto name : all_algos)
    {
        DYNAMIC_SECTION(name)
        {
            auto algo = lookup_algo(mod, std::string{name});
            auto compress = lookup_fn(algo, "compress");
            auto decompress = lookup_fn(algo, "decompress");

            auto input = Value::create("hello hello hello hello hello"s);
            auto compressed = call1(compress, input);
            REQUIRE(compressed->is<String>());

            auto decompressed = call1(decompress, compressed);
            REQUIRE(decompressed->is<String>());
            CHECK(decompressed->raw_get<String>() == "hello hello hello hello hello");
        }
    }
}

TEST_CASE("ext::compression: round-trip empty string")
{
    auto mod = compression_module();

    for (auto name : all_algos)
    {
        DYNAMIC_SECTION(name)
        {
            auto algo = lookup_algo(mod, std::string{name});
            auto compress = lookup_fn(algo, "compress");
            auto decompress = lookup_fn(algo, "decompress");

            auto compressed = call1(compress, Value::create(""s));
            auto decompressed = call1(decompress, compressed);
            CHECK(decompressed->raw_get<String>().empty());
        }
    }
}

TEST_CASE("ext::compression: round-trip binary data")
{
    auto mod = compression_module();

    for (auto name : all_algos)
    {
        DYNAMIC_SECTION(name)
        {
            auto algo = lookup_algo(mod, std::string{name});
            auto compress = lookup_fn(algo, "compress");
            auto decompress = lookup_fn(algo, "decompress");

            // String with null bytes and high bytes
            std::string binary = "\x00\x01\x02\xff\xfe\xfd"s;
            auto compressed = call1(compress, Value::create(String{binary}));
            auto decompressed = call1(decompress, compressed);
            CHECK(decompressed->raw_get<String>() == binary);
        }
    }
}

TEST_CASE("ext::compression: zlib round-trip with explicit level")
{
    auto mod = compression_module();

    for (auto name : zlib_algos)
    {
        DYNAMIC_SECTION(name)
        {
            auto algo = lookup_algo(mod, std::string{name});
            auto compress = lookup_fn(algo, "compress");
            auto decompress = lookup_fn(algo, "decompress");

            auto input = Value::create("aaaaaaaaaa"s);

            for (Int level : {-1_f, 0_f, 1_f, 6_f, 9_f})
            {
                auto compressed =
                    call2(compress, input, Value::create(level));
                auto decompressed = call1(decompress, compressed);
                CHECK(decompressed->raw_get<String>() == "aaaaaaaaaa");
            }
        }
    }
}

TEST_CASE("ext::compression: brotli round-trip with explicit quality")
{
    auto mod = compression_module();
    auto algo = lookup_algo(mod, "brotli");
    auto compress = lookup_fn(algo, "compress");
    auto decompress = lookup_fn(algo, "decompress");

    auto input = Value::create("aaaaaaaaaa"s);

    for (Int quality : {0_f, 1_f, 6_f, 11_f})
    {
        auto compressed = call2(compress, input, Value::create(quality));
        auto decompressed = call1(decompress, compressed);
        CHECK(decompressed->raw_get<String>() == "aaaaaaaaaa");
    }
}

// =============================================================================
// Compression actually reduces size
// =============================================================================

TEST_CASE("ext::compression: compresses repetitive data")
{
    auto mod = compression_module();

    for (auto name : all_algos)
    {
        DYNAMIC_SECTION(name)
        {
            auto compress =
                lookup_fn(lookup_algo(mod, std::string{name}), "compress");

            std::string repetitive(1000, 'x');
            auto compressed =
                call1(compress, Value::create(String{repetitive}));
            CHECK(compressed->raw_get<String>().size() < 100);
        }
    }
}

// =============================================================================
// Cross-format rejection
// =============================================================================

TEST_CASE("ext::compression: cross-format decompression fails")
{
    auto mod = compression_module();
    auto deflate_compress =
        lookup_fn(lookup_algo(mod, "deflate"), "compress");
    auto gzip_decompress =
        lookup_fn(lookup_algo(mod, "gzip"), "decompress");
    auto zlib_decompress =
        lookup_fn(lookup_algo(mod, "zlib"), "decompress");
    auto brotli_decompress =
        lookup_fn(lookup_algo(mod, "brotli"), "decompress");

    auto deflated = call1(deflate_compress, Value::create("test"s));

    CHECK_THROWS_WITH(call1(gzip_decompress, deflated),
                      ContainsSubstring("decompression failed"));
    CHECK_THROWS_WITH(call1(zlib_decompress, deflated),
                      ContainsSubstring("decompression failed"));
    CHECK_THROWS_WITH(call1(brotli_decompress, deflated),
                      ContainsSubstring("decompression failed"));
}

// =============================================================================
// Corrupt input
// =============================================================================

TEST_CASE("ext::compression: corrupt input")
{
    auto mod = compression_module();

    for (auto name : all_algos)
    {
        DYNAMIC_SECTION(name)
        {
            auto decompress =
                lookup_fn(lookup_algo(mod, std::string{name}), "decompress");
            CHECK_THROWS_WITH(
                call1(decompress, Value::create("not compressed data"s)),
                ContainsSubstring("decompression failed"));
        }
    }
}
