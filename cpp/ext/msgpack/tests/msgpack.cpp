#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/extensions-common.hpp>
#include <frost/value.hpp>

#include <msgpack.hpp>

using namespace frst;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;

namespace frst
{
void register_module_msgpack(Stdlib_Registry_Builder&);
}

namespace
{

Map msgpack_module()
{
    Stdlib_Registry_Builder builder;
    register_module_msgpack(builder);
    auto registry = std::move(builder).build();
    auto module = registry.lookup_module("ext.msgpack");
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

// Helper: pack a single msgpack value into a String
std::string pack_value(auto fn)
{
    ::msgpack::sbuffer buf;
    ::msgpack::packer pk{buf};
    fn(pk);
    return {buf.data(), buf.size()};
}

} // namespace

TEST_CASE("msgpack.decode: primitives")
{
    auto mod = msgpack_module();
    auto decode = lookup(mod, "decode");

    SECTION("nil")
    {
        auto data = pack_value([](auto& pk) { pk.pack_nil(); });
        auto result = decode->call({Value::create(String{data})});
        CHECK(result == Value::null());
    }

    SECTION("true")
    {
        auto data = pack_value([](auto& pk) { pk.pack_true(); });
        auto result = decode->call({Value::create(String{data})});
        CHECK(result->raw_get<Bool>() == true);
    }

    SECTION("false")
    {
        auto data = pack_value([](auto& pk) { pk.pack_false(); });
        auto result = decode->call({Value::create(String{data})});
        CHECK(result->raw_get<Bool>() == false);
    }

    SECTION("positive integer")
    {
        auto data = pack_value([](auto& pk) { pk.pack_int64(42); });
        auto result = decode->call({Value::create(String{data})});
        CHECK(result->raw_get<Int>() == 42);
    }

    SECTION("negative integer")
    {
        auto data = pack_value([](auto& pk) { pk.pack_int64(-100); });
        auto result = decode->call({Value::create(String{data})});
        CHECK(result->raw_get<Int>() == -100);
    }

    SECTION("uint64 overflow rejected")
    {
        auto data = pack_value([](auto& pk) {
            pk.pack_uint64(static_cast<uint64_t>(
                std::numeric_limits<int64_t>::max()) + 1);
        });
        CHECK_THROWS_WITH(decode->call({Value::create(String{data})}),
                          ContainsSubstring("exceeds Int range"));
    }

    SECTION("float")
    {
        auto data = pack_value([](auto& pk) { pk.pack_double(3.14); });
        auto result = decode->call({Value::create(String{data})});
        CHECK(result->raw_get<Float>() == Catch::Approx(3.14));
    }

    SECTION("string")
    {
        auto data = pack_value([](auto& pk) {
            pk.pack_str(5);
            pk.pack_str_body("hello", 5);
        });
        auto result = decode->call({Value::create(String{data})});
        CHECK(result->raw_get<String>() == "hello");
    }

    SECTION("binary")
    {
        auto data = pack_value([](auto& pk) {
            pk.pack_bin(3);
            pk.pack_bin_body("\x01\x02\x03", 3);
        });
        auto result = decode->call({Value::create(String{data})});
        REQUIRE(result->is<Function>());
        auto bytes = result->raw_get<Function>()->call({});
        CHECK(bytes->raw_get<String>() == "\x01\x02\x03"s);
    }
}

TEST_CASE("msgpack.decode: structures")
{
    auto mod = msgpack_module();
    auto decode = lookup(mod, "decode");

    SECTION("array")
    {
        auto data = pack_value([](auto& pk) {
            pk.pack_array(3);
            pk.pack_int64(1);
            pk.pack_int64(2);
            pk.pack_int64(3);
        });
        auto result = decode->call({Value::create(String{data})});
        REQUIRE(result->is<Array>());
        const auto& arr = result->raw_get<Array>();
        REQUIRE(arr.size() == 3);
        CHECK(arr[0]->raw_get<Int>() == 1);
        CHECK(arr[1]->raw_get<Int>() == 2);
        CHECK(arr[2]->raw_get<Int>() == 3);
    }

    SECTION("map")
    {
        auto data = pack_value([](auto& pk) {
            pk.pack_map(2);
            pk.pack_str(1);
            pk.pack_str_body("a", 1);
            pk.pack_int64(1);
            pk.pack_str(1);
            pk.pack_str_body("b", 1);
            pk.pack_int64(2);
        });
        auto result = decode->call({Value::create(String{data})});
        REQUIRE(result->is<Map>());
        const auto& map = result->raw_get<Map>();
        CHECK(map.at(Value::create("a"s))->raw_get<Int>() == 1);
        CHECK(map.at(Value::create("b"s))->raw_get<Int>() == 2);
    }

    SECTION("nested")
    {
        auto data = pack_value([](auto& pk) {
            pk.pack_map(1);
            pk.pack_str(4);
            pk.pack_str_body("data", 4);
            pk.pack_array(2);
            pk.pack_int64(1);
            pk.pack_true();
        });
        auto result = decode->call({Value::create(String{data})});
        REQUIRE(result->is<Map>());
        auto inner =
            result->raw_get<Map>().at(Value::create("data"s));
        REQUIRE(inner->is<Array>());
        CHECK(inner->raw_get<Array>()[0]->raw_get<Int>() == 1);
        CHECK(inner->raw_get<Array>()[1]->raw_get<Bool>() == true);
    }
}

TEST_CASE("msgpack.decode: special floats")
{
    auto mod = msgpack_module();
    auto decode = lookup(mod, "decode");

    SECTION("nan")
    {
        auto data = pack_value(
            [](auto& pk) { pk.pack_double(std::numeric_limits<double>::quiet_NaN()); });
        auto result = decode->call({Value::create(String{data})});
        REQUIRE(result->is<Function>());
        CHECK(result->raw_get<Function>()->call({})->raw_get<String>() == "nan");
    }

    SECTION("inf")
    {
        auto data = pack_value(
            [](auto& pk) { pk.pack_double(std::numeric_limits<double>::infinity()); });
        auto result = decode->call({Value::create(String{data})});
        REQUIRE(result->is<Function>());
        CHECK(result->raw_get<Function>()->call({})->raw_get<String>() == "inf");
    }

    SECTION("-inf")
    {
        auto data = pack_value(
            [](auto& pk) { pk.pack_double(-std::numeric_limits<double>::infinity()); });
        auto result = decode->call({Value::create(String{data})});
        REQUIRE(result->is<Function>());
        CHECK(result->raw_get<Function>()->call({})->raw_get<String>()
              == "-inf");
    }
}

TEST_CASE("msgpack.decode: error handling")
{
    auto mod = msgpack_module();
    auto decode = lookup(mod, "decode");

    SECTION("invalid data")
    {
        CHECK_THROWS_WITH(decode->call({Value::create("\xc1"s)}),
                          ContainsSubstring("msgpack.decode"));
    }

    SECTION("extension type rejected")
    {
        auto data = pack_value([](auto& pk) {
            pk.pack_ext(1, 1);
            pk.pack_ext_body("\x00", 1);
        });
        CHECK_THROWS_WITH(decode->call({Value::create(String{data})}),
                          ContainsSubstring("extension type"));
    }

    SECTION("null map key rejected")
    {
        auto data = pack_value([](auto& pk) {
            pk.pack_map(1);
            pk.pack_nil();
            pk.pack_int64(1);
        });
        CHECK_THROWS_WITH(decode->call({Value::create(String{data})}),
                          ContainsSubstring("map key type"));
    }

    SECTION("array map key rejected")
    {
        auto data = pack_value([](auto& pk) {
            pk.pack_map(1);
            pk.pack_array(0);
            pk.pack_int64(1);
        });
        CHECK_THROWS_WITH(decode->call({Value::create(String{data})}),
                          ContainsSubstring("map key type"));
    }

    SECTION("arity")
    {
        CHECK_THROWS_WITH(decode->call({}),
                          ContainsSubstring("insufficient"));
        CHECK_THROWS_WITH(
            decode->call({Value::create("a"s), Value::create("b"s)}),
            ContainsSubstring("too many"));
    }

    SECTION("type constraint")
    {
        CHECK_THROWS_WITH(decode->call({Value::create(42_f)}),
                          ContainsSubstring("String"));
    }
}

TEST_CASE("msgpack.encode: primitives")
{
    auto mod = msgpack_module();
    auto encode = lookup(mod, "encode");
    auto decode = lookup(mod, "decode");

    SECTION("null round-trips")
    {
        auto encoded = encode->call({Value::null()});
        auto decoded = decode->call({encoded});
        CHECK(decoded == Value::null());
    }

    SECTION("bool round-trips")
    {
        auto encoded = encode->call({Value::create(true)});
        auto decoded = decode->call({encoded});
        CHECK(decoded->raw_get<Bool>() == true);
    }

    SECTION("int round-trips")
    {
        auto encoded = encode->call({Value::create(42_f)});
        auto decoded = decode->call({encoded});
        CHECK(decoded->raw_get<Int>() == 42);
    }

    SECTION("negative int round-trips")
    {
        auto encoded = encode->call({Value::create(-999_f)});
        auto decoded = decode->call({encoded});
        CHECK(decoded->raw_get<Int>() == -999);
    }

    SECTION("float round-trips")
    {
        auto encoded = encode->call({Value::create(3.14)});
        auto decoded = decode->call({encoded});
        CHECK(decoded->raw_get<Float>() == Catch::Approx(3.14));
    }

    SECTION("string round-trips")
    {
        auto encoded = encode->call({Value::create("hello"s)});
        auto decoded = decode->call({encoded});
        CHECK(decoded->raw_get<String>() == "hello");
    }
}

TEST_CASE("msgpack.encode: structures")
{
    auto mod = msgpack_module();
    auto encode = lookup(mod, "encode");
    auto decode = lookup(mod, "decode");

    SECTION("array round-trips")
    {
        auto arr = Value::create(
            Array{Value::create(1_f), Value::create(2_f), Value::create(3_f)});
        auto decoded = decode->call({encode->call({arr})});
        REQUIRE(decoded->is<Array>());
        const auto& darr = decoded->raw_get<Array>();
        REQUIRE(darr.size() == 3);
        CHECK(darr[0]->raw_get<Int>() == 1);
        CHECK(darr[1]->raw_get<Int>() == 2);
        CHECK(darr[2]->raw_get<Int>() == 3);
    }

    SECTION("map round-trips")
    {
        auto map = Value::create(Value::trusted,
                                 Map{{Value::create("x"s), Value::create(1_f)},
                                     {Value::create("y"s), Value::create(2_f)}});
        auto decoded = decode->call({encode->call({map})});
        REQUIRE(decoded->is<Map>());
        CHECK(decoded->raw_get<Map>().at(Value::create("x"s))->raw_get<Int>()
              == 1);
        CHECK(decoded->raw_get<Map>().at(Value::create("y"s))->raw_get<Int>()
              == 2);
    }
}

TEST_CASE("msgpack.encode: cannot serialize plain Function")
{
    auto mod = msgpack_module();
    auto encode = lookup(mod, "encode");

    auto fn = Value::create(
        Function{std::make_shared<Builtin>(
            [](builtin_args_t) { return Value::null(); }, "dummy")});
    CHECK_THROWS_WITH(encode->call({fn}),
                      ContainsSubstring("cannot serialize Function"));
}

TEST_CASE("msgpack.binary: constructor")
{
    auto mod = msgpack_module();
    auto binary_fn = lookup(mod, "binary");

    SECTION("creates foreign value")
    {
        auto result = binary_fn->call({Value::create("hello"s)});
        REQUIRE(result->is<Function>());
        CHECK(result->raw_get<Function>()->call({})->raw_get<String>()
              == "hello");
    }

    SECTION("arity")
    {
        CHECK_THROWS_WITH(binary_fn->call({}),
                          ContainsSubstring("insufficient"));
        CHECK_THROWS_WITH(
            binary_fn->call({Value::create("a"s), Value::create("b"s)}),
            ContainsSubstring("too many"));
    }

    SECTION("type constraint")
    {
        CHECK_THROWS_WITH(binary_fn->call({Value::create(42_f)}),
                          ContainsSubstring("String"));
    }
}

TEST_CASE("msgpack.special_float: constructor")
{
    auto mod = msgpack_module();
    auto sf_fn = lookup(mod, "special_float");

    SECTION("nan")
    {
        auto result = sf_fn->call({Value::create("nan"s)});
        REQUIRE(result->is<Function>());
        CHECK(result->raw_get<Function>()->call({})->raw_get<String>() == "nan");
    }

    SECTION("inf")
    {
        auto result = sf_fn->call({Value::create("inf"s)});
        REQUIRE(result->is<Function>());
        CHECK(result->raw_get<Function>()->call({})->raw_get<String>() == "inf");
    }

    SECTION("-inf")
    {
        auto result = sf_fn->call({Value::create("-inf"s)});
        REQUIRE(result->is<Function>());
        CHECK(result->raw_get<Function>()->call({})->raw_get<String>()
              == "-inf");
    }

    SECTION("invalid string")
    {
        CHECK_THROWS_WITH(sf_fn->call({Value::create("NaN"s)}),
                          ContainsSubstring("nan"));
    }

    SECTION("arity")
    {
        CHECK_THROWS_WITH(sf_fn->call({}),
                          ContainsSubstring("insufficient"));
    }
}
