#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_exception.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/stdlib.hpp>
#include <frost/value.hpp>

using namespace frst;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;
using Catch::Matchers::MessageMatches;

namespace
{

Value_Ptr make_module(std::initializer_list<std::pair<String, int>> entries)
{
    Map m;
    for (auto& [name, val] : entries)
        m.emplace(Value::create(String{name}), Value::create(Int{val}));
    return Value::create(Value::trusted, std::move(m));
}

} // namespace

// --- Builder: register_module ---

TEST_CASE("Builder registers a module")
{
    Stdlib_Registry_Builder builder;
    builder.register_module(
        Stdlib_Registry_Builder::module_path_t({"std", "test"}),
        make_module({{"a", 1}}));
    auto registry = std::move(builder).build();

    auto result = registry.lookup_module("std.test");
    REQUIRE(result.has_value());
    REQUIRE(result.value()->is<Map>());
}

TEST_CASE("Builder groups modules under the same namespace", "")
{
    Stdlib_Registry_Builder builder;
    builder.register_module(
        Stdlib_Registry_Builder::module_path_t({"std", "alpha"}),
        make_module({{"a", 1}}));
    builder.register_module(
        Stdlib_Registry_Builder::module_path_t({"std", "beta"}),
        make_module({{"b", 2}}));
    auto registry = std::move(builder).build();

    REQUIRE(registry.lookup_module("std.alpha").has_value());
    REQUIRE(registry.lookup_module("std.beta").has_value());
}

TEST_CASE("Builder supports multiple namespaces")
{
    Stdlib_Registry_Builder builder;
    builder.register_module(
        Stdlib_Registry_Builder::module_path_t({"std", "core"}),
        make_module({{"x", 1}}));
    builder.register_module(
        Stdlib_Registry_Builder::module_path_t({"ext", "plugin"}),
        make_module({{"y", 2}}));
    auto registry = std::move(builder).build();

    REQUIRE(registry.lookup_module("std.core").has_value());
    REQUIRE(registry.lookup_module("ext.plugin").has_value());
}

// --- Builder: error cases ---

TEST_CASE("Builder rejects empty namespace segment")
{
    Stdlib_Registry_Builder builder;
    REQUIRE_THROWS_MATCHES(
        builder.register_module(
            Stdlib_Registry_Builder::module_path_t({"", "test"}),
            Value::null()),
        Frost_Interpreter_Error, MessageMatches(ContainsSubstring("Empty")));
}

TEST_CASE("Builder rejects empty module segment")
{
    Stdlib_Registry_Builder builder;
    REQUIRE_THROWS_MATCHES(
        builder.register_module(
            Stdlib_Registry_Builder::module_path_t({"std", ""}), Value::null()),
        Frost_Interpreter_Error, MessageMatches(ContainsSubstring("Empty")));
}

TEST_CASE("Builder rejects duplicate registration")
{
    Stdlib_Registry_Builder builder;
    builder.register_module(
        Stdlib_Registry_Builder::module_path_t({"std", "dup"}),
        make_module({{"a", 1}}));

    REQUIRE_THROWS_MATCHES(
        builder.register_module(
            Stdlib_Registry_Builder::module_path_t({"std", "dup"}),
            make_module({{"b", 2}})),
        Frost_Interpreter_Error,
        MessageMatches(ContainsSubstring("Duplicate")));
}

// --- Lookup: namespace level ---

TEST_CASE("Lookup of a namespace returns the namespace Map", "")
{
    Stdlib_Registry_Builder builder;
    builder.register_module(
        Stdlib_Registry_Builder::module_path_t({"std", "foo"}),
        make_module({{"x", 1}}));
    builder.register_module(
        Stdlib_Registry_Builder::module_path_t({"std", "bar"}),
        make_module({{"y", 2}}));
    auto registry = std::move(builder).build();

    auto ns = registry.lookup_module("std");
    REQUIRE(ns.has_value());
    REQUIRE(ns.value()->is<Map>());

    // The namespace Map should contain both modules
    const auto& ns_map = ns.value()->raw_get<Map>();
    REQUIRE(ns_map.size() == 2);
}

// --- Lookup: deep paths ---

TEST_CASE("Lookup reaches into a module's sub-maps")
{
    // Build a module with a nested sub-map, like encoding has b64
    Map inner;
    inner.emplace(Value::create("encode"s), Value::create(Int{42}));
    Map module;
    module.emplace(Value::create("sub"s),
                   Value::create(Value::trusted, std::move(inner)));
    module.emplace(Value::create("top_fn"s), Value::create(Int{99}));

    Stdlib_Registry_Builder builder;
    builder.register_module(
        Stdlib_Registry_Builder::module_path_t({"std", "nested"}),
        Value::create(Value::trusted, std::move(module)));
    auto registry = std::move(builder).build();

    // Two-segment: the module itself
    auto mod = registry.lookup_module("std.nested");
    REQUIRE(mod.has_value());
    REQUIRE(mod.value()->is<Map>());

    // Three-segment: the sub-map
    auto sub = registry.lookup_module("std.nested.sub");
    REQUIRE(sub.has_value());
    REQUIRE(sub.value()->is<Map>());

    // Four-segment: a leaf value inside the sub-map
    auto leaf = registry.lookup_module("std.nested.sub.encode");
    REQUIRE(leaf.has_value());
    REQUIRE(leaf.value()->is<Int>());
    CHECK(leaf.value()->raw_get<Int>() == 42);

    // Three-segment: a leaf value at module level
    auto top = registry.lookup_module("std.nested.top_fn");
    REQUIRE(top.has_value());
    CHECK(top.value()->raw_get<Int>() == 99);
}

// --- Lookup: miss cases ---

TEST_CASE("Lookup returns nullopt for missing namespace")
{
    Stdlib_Registry_Builder builder;
    builder.register_module(
        Stdlib_Registry_Builder::module_path_t({"std", "only"}),
        make_module({{"a", 1}}));
    auto registry = std::move(builder).build();

    CHECK_FALSE(registry.lookup_module("ext.only").has_value());
}

TEST_CASE("Lookup returns nullopt for missing module")
{
    Stdlib_Registry_Builder builder;
    builder.register_module(
        Stdlib_Registry_Builder::module_path_t({"std", "real"}),
        make_module({{"a", 1}}));
    auto registry = std::move(builder).build();

    CHECK_FALSE(registry.lookup_module("std.fake").has_value());
}

TEST_CASE("Lookup returns nullopt when traversing through a non-Map", "")
{
    Stdlib_Registry_Builder builder;
    builder.register_module(
        Stdlib_Registry_Builder::module_path_t({"std", "flat"}),
        make_module({{"val", 1}}));
    auto registry = std::move(builder).build();

    // "val" is an Int, so "val.deeper" should fail
    CHECK_FALSE(registry.lookup_module("std.flat.val.deeper").has_value());
}

TEST_CASE("Lookup returns nullopt for missing deep key")
{
    Stdlib_Registry_Builder builder;
    builder.register_module(
        Stdlib_Registry_Builder::module_path_t({"std", "mod"}),
        make_module({{"a", 1}}));
    auto registry = std::move(builder).build();

    CHECK_FALSE(registry.lookup_module("std.mod.nonexistent").has_value());
}

// --- register_stdlib produces a working registry ---

TEST_CASE("register_stdlib populates all standard modules", "")
{
    Stdlib_Registry_Builder builder;
    register_stdlib(builder);
    auto registry = std::move(builder).build();

    CHECK(registry.lookup_module("std.encoding").has_value());
    CHECK(registry.lookup_module("std.fs").has_value());
    CHECK(registry.lookup_module("std.io").has_value());
    CHECK(registry.lookup_module("std.json").has_value());
    CHECK(registry.lookup_module("std.math").has_value());
    CHECK(registry.lookup_module("std.hash").has_value());
    CHECK(registry.lookup_module("std.os").has_value());
    CHECK(registry.lookup_module("std.regex").has_value());

    // Namespace-level lookup returns a Map containing all modules
    auto std_ns = registry.lookup_module("std");
    REQUIRE(std_ns.has_value());
    REQUIRE(std_ns.value()->is<Map>());
    CHECK(std_ns.value()->raw_get<Map>().size() == 8);
}

TEST_CASE("register_stdlib deep path into encoding.b64")
{
    Stdlib_Registry_Builder builder;
    register_stdlib(builder);
    auto registry = std::move(builder).build();

    auto b64 = registry.lookup_module("std.encoding.b64");
    REQUIRE(b64.has_value());
    REQUIRE(b64.value()->is<Map>());

    // b64 sub-map should contain encode, decode, urlencode, urldecode
    const auto& b64_map = b64.value()->raw_get<Map>();
    CHECK(b64_map.size() == 4);
}
