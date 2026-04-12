#include <catch2/catch_all.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <frost/builtin.hpp>

using namespace frst;
using namespace std::literals;
using namespace Catch::Matchers;

namespace
{

Function get_fn(Symbol_Table& table, const std::string& name)
{
    auto val = table.lookup(name);
    REQUIRE(val->is<Function>());
    return val->get<Function>().value();
}

Value_Ptr call1(const Function& fn, Value_Ptr arg)
{
    std::vector<Value_Ptr> args{std::move(arg)};
    return fn->call(args);
}

// Convenience: {key: k, value: v}
Value_Ptr entry(Value_Ptr k, Value_Ptr v)
{
    return Value::create(Value::trusted,
                         Map{{Value::create("key"s), std::move(k)},
                             {Value::create("value"s), std::move(v)}});
}

} // namespace

// =============================================================================
// to_entries
// =============================================================================

TEST_CASE("to_entries: arity")
{
    Symbol_Table table;
    inject_builtins(table);
    auto to_entries = get_fn(table, "to_entries");

    CHECK_THROWS_WITH(to_entries->call({}),
                      ContainsSubstring("insufficient arguments"));
    CHECK_THROWS_WITH(
        to_entries->call({Value::create(Map{}), Value::create(Map{})}),
        ContainsSubstring("too many arguments"));
}

TEST_CASE("to_entries: wrong type")
{
    Symbol_Table table;
    inject_builtins(table);
    auto to_entries = get_fn(table, "to_entries");

    CHECK_THROWS(call1(to_entries, Value::null()));
    CHECK_THROWS(call1(to_entries, Value::create(42_f)));
    CHECK_THROWS(call1(to_entries, Value::create("hi"s)));
    CHECK_THROWS(call1(to_entries, Value::create(Array{})));
}

TEST_CASE("to_entries: empty map")
{
    Symbol_Table table;
    inject_builtins(table);
    auto to_entries = get_fn(table, "to_entries");

    auto result = call1(to_entries, Value::create(Map{}));
    REQUIRE(result->is<Array>());
    CHECK(result->raw_get<Array>().empty());
}

TEST_CASE("to_entries: string-keyed map")
{
    Symbol_Table table;
    inject_builtins(table);
    auto to_entries = get_fn(table, "to_entries");

    auto map = Value::create(Map{
        {Value::create("a"s), Value::create(1_f)},
        {Value::create("b"s), Value::create(2_f)},
    });

    auto result = call1(to_entries, map);
    REQUIRE(result->is<Array>());
    const auto& arr = result->raw_get<Array>();
    REQUIRE(arr.size() == 2);

    // Each entry must be a {key: ..., value: ...} map. Verify both entries
    // are present without assuming iteration order.
    auto key_str = Value::create("key"s);
    auto val_str = Value::create("value"s);
    bool found_a = false;
    bool found_b = false;
    for (const auto& elem : arr)
    {
        REQUIRE(elem->is<Map>());
        const auto& e = elem->raw_get<Map>();
        auto k = e.find(key_str)->second;
        auto v = e.find(val_str)->second;
        if (k->raw_get<String>() == "a")
        {
            CHECK(v->get<Int>().value() == 1_f);
            found_a = true;
        }
        else if (k->raw_get<String>() == "b")
        {
            CHECK(v->get<Int>().value() == 2_f);
            found_b = true;
        }
    }
    CHECK(found_a);
    CHECK(found_b);
}

TEST_CASE("to_entries: non-string keys")
{
    Symbol_Table table;
    inject_builtins(table);
    auto to_entries = get_fn(table, "to_entries");

    auto map = Value::create(Map{
        {Value::create(1_f), Value::create("one"s)},
        {Value::create(true), Value::create("yes"s)},
    });

    auto result = call1(to_entries, map);
    REQUIRE(result->is<Array>());
    CHECK(result->raw_get<Array>().size() == 2);

    // Each entry should be a map with "key" and "value" fields.
    for (const auto& elem : result->raw_get<Array>())
    {
        REQUIRE(elem->is<Map>());
        const auto& e = elem->raw_get<Map>();
        CHECK(e.find(Value::create("key"s)) != e.end());
        CHECK(e.find(Value::create("value"s)) != e.end());
    }
}

TEST_CASE("to_entries: values can be any type")
{
    Symbol_Table table;
    inject_builtins(table);
    auto to_entries = get_fn(table, "to_entries");

    auto map = Value::create(Map{
        {Value::create("n"s), Value::null()},
        {Value::create("a"s), Value::create(Array{Value::create(1_f)})},
    });

    auto result = call1(to_entries, map);
    REQUIRE(result->is<Array>());
    CHECK(result->raw_get<Array>().size() == 2);

    auto key_str = Value::create("key"s);
    auto val_str = Value::create("value"s);
    for (const auto& elem : result->raw_get<Array>())
    {
        REQUIRE(elem->is<Map>());
        const auto& e = elem->raw_get<Map>();
        auto k = e.find(key_str)->second;
        auto v = e.find(val_str)->second;
        if (k->raw_get<String>() == "n")
            CHECK(v->is<Null>());
        else if (k->raw_get<String>() == "a")
            CHECK(v->is<Array>());
    }
}

// =============================================================================
// from_entries
// =============================================================================

TEST_CASE("from_entries: arity")
{
    Symbol_Table table;
    inject_builtins(table);
    auto from_entries = get_fn(table, "from_entries");

    CHECK_THROWS_WITH(from_entries->call({}),
                      ContainsSubstring("insufficient arguments"));
    CHECK_THROWS_WITH(
        from_entries->call({Value::create(Array{}), Value::create(Array{})}),
        ContainsSubstring("too many arguments"));
}

TEST_CASE("from_entries: wrong type")
{
    Symbol_Table table;
    inject_builtins(table);
    auto from_entries = get_fn(table, "from_entries");

    CHECK_THROWS(call1(from_entries, Value::null()));
    CHECK_THROWS(call1(from_entries, Value::create(42_f)));
    CHECK_THROWS(call1(from_entries, Value::create("hi"s)));
    CHECK_THROWS(call1(from_entries, Value::create(Map{})));
}

TEST_CASE("from_entries: empty array")
{
    Symbol_Table table;
    inject_builtins(table);
    auto from_entries = get_fn(table, "from_entries");

    auto result = call1(from_entries, Value::create(Array{}));
    REQUIRE(result->is<Map>());
    CHECK(result->raw_get<Map>().empty());
}

TEST_CASE("from_entries: basic reconstruction")
{
    Symbol_Table table;
    inject_builtins(table);
    auto from_entries = get_fn(table, "from_entries");

    auto entries = Value::create(Array{
        entry(Value::create("a"s), Value::create(1_f)),
        entry(Value::create("b"s), Value::create(2_f)),
    });

    auto result = call1(from_entries, entries);
    REQUIRE(result->is<Map>());
    const auto& m = result->raw_get<Map>();
    REQUIRE(m.size() == 2);
    CHECK(m.find(Value::create("a"s))->second->get<Int>().value() == 1_f);
    CHECK(m.find(Value::create("b"s))->second->get<Int>().value() == 2_f);
}

TEST_CASE("from_entries: non-string keys")
{
    Symbol_Table table;
    inject_builtins(table);
    auto from_entries = get_fn(table, "from_entries");

    auto entries = Value::create(Array{
        entry(Value::create(1_f), Value::create("one"s)),
        entry(Value::create(true), Value::create("yes"s)),
    });

    auto result = call1(from_entries, entries);
    REQUIRE(result->is<Map>());
    const auto& m = result->raw_get<Map>();
    CHECK(m.find(Value::create(1_f))->second->raw_get<String>() == "one");
    CHECK(m.find(Value::create(true))->second->raw_get<String>() == "yes");
}

TEST_CASE("from_entries: duplicate keys (last wins)")
{
    Symbol_Table table;
    inject_builtins(table);
    auto from_entries = get_fn(table, "from_entries");

    auto entries = Value::create(Array{
        entry(Value::create("k"s), Value::create(1_f)),
        entry(Value::create("k"s), Value::create(2_f)),
    });

    auto result = call1(from_entries, entries);
    REQUIRE(result->is<Map>());
    const auto& m = result->raw_get<Map>();
    REQUIRE(m.size() == 1);
    CHECK(m.find(Value::create("k"s))->second->get<Int>().value() == 2_f);
}

TEST_CASE("from_entries: extra keys in entry maps are ignored")
{
    Symbol_Table table;
    inject_builtins(table);
    auto from_entries = get_fn(table, "from_entries");

    // Entry has key, value, and an extra "name" field -- should be fine.
    auto entries = Value::create(Array{
        Value::create(Map{
            {Value::create("key"s), Value::create("a"s)},
            {Value::create("value"s), Value::create(1_f)},
            {Value::create("name"s), Value::create("extra"s)},
        }),
    });

    auto result = call1(from_entries, entries);
    REQUIRE(result->is<Map>());
    CHECK(result->raw_get<Map>().size() == 1);
    CHECK(result->raw_get<Map>().find(Value::create("a"s))->second->get<Int>().value() == 1_f);
}

TEST_CASE("from_entries: element not a map")
{
    Symbol_Table table;
    inject_builtins(table);
    auto from_entries = get_fn(table, "from_entries");

    auto bad = Value::create(Array{Value::create(42_f)});
    CHECK_THROWS_WITH(call1(from_entries, bad),
                      ContainsSubstring("element 0") && ContainsSubstring("Map"));
}

TEST_CASE("from_entries: missing 'key'")
{
    Symbol_Table table;
    inject_builtins(table);
    auto from_entries = get_fn(table, "from_entries");

    auto no_key = Value::create(Array{
        Value::create(Map{{Value::create("value"s), Value::create(1_f)}}),
    });
    CHECK_THROWS_WITH(call1(from_entries, no_key),
                      ContainsSubstring("missing 'key'"));
}

TEST_CASE("from_entries: missing 'value'")
{
    Symbol_Table table;
    inject_builtins(table);
    auto from_entries = get_fn(table, "from_entries");

    auto no_val = Value::create(Array{
        Value::create(Map{{Value::create("key"s), Value::create("a"s)}}),
    });
    CHECK_THROWS_WITH(call1(from_entries, no_val),
                      ContainsSubstring("missing 'value'"));
}

TEST_CASE("from_entries: invalid key type (null)")
{
    Symbol_Table table;
    inject_builtins(table);
    auto from_entries = get_fn(table, "from_entries");

    auto null_key = Value::create(Array{
        entry(Value::null(), Value::create(1_f)),
    });
    CHECK_THROWS_WITH(call1(from_entries, null_key),
                      ContainsSubstring("invalid key type"));
}

TEST_CASE("from_entries: invalid key type (Array)")
{
    Symbol_Table table;
    inject_builtins(table);
    auto from_entries = get_fn(table, "from_entries");

    auto arr_key = Value::create(Array{
        entry(Value::create(Array{}), Value::create(1_f)),
    });
    CHECK_THROWS_WITH(call1(from_entries, arr_key),
                      ContainsSubstring("invalid key type"));
}

// =============================================================================
// Round-trip: to_entries and from_entries are inverses
// =============================================================================

TEST_CASE("to_entries -> from_entries round-trip")
{
    Symbol_Table table;
    inject_builtins(table);
    auto to_entries = get_fn(table, "to_entries");
    auto from_entries = get_fn(table, "from_entries");

    auto original = Value::create(Map{
        {Value::create("x"s), Value::create(10_f)},
        {Value::create("y"s), Value::create(20_f)},
        {Value::create("z"s), Value::create(30_f)},
    });

    auto entries = call1(to_entries, original);
    auto reconstructed = call1(from_entries, entries);

    CHECK(Value::equal(original, reconstructed)->truthy());
}

TEST_CASE("from_entries -> to_entries round-trip")
{
    Symbol_Table table;
    inject_builtins(table);
    auto to_entries = get_fn(table, "to_entries");
    auto from_entries = get_fn(table, "from_entries");

    auto entries = Value::create(Array{
        entry(Value::create("a"s), Value::create(1_f)),
        entry(Value::create("b"s), Value::create(2_f)),
    });

    auto map = call1(from_entries, entries);
    auto back = call1(to_entries, map);

    CHECK(Value::equal(entries, back)->truthy());
}

TEST_CASE("round-trip with non-string keys")
{
    Symbol_Table table;
    inject_builtins(table);
    auto to_entries = get_fn(table, "to_entries");
    auto from_entries = get_fn(table, "from_entries");

    auto original = Value::create(Map{
        {Value::create(1_f), Value::create("one"s)},
        {Value::create(2_f), Value::create("two"s)},
        {Value::create(true), Value::create("yes"s)},
    });

    auto reconstructed = call1(from_entries, call1(to_entries, original));
    CHECK(Value::equal(original, reconstructed)->truthy());
}

TEST_CASE("round-trip preserves empty map")
{
    Symbol_Table table;
    inject_builtins(table);
    auto to_entries = get_fn(table, "to_entries");
    auto from_entries = get_fn(table, "from_entries");

    auto empty = Value::create(Map{});
    auto reconstructed = call1(from_entries, call1(to_entries, empty));
    CHECK(Value::equal(empty, reconstructed)->truthy());
}
