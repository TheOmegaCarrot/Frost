#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/stdlib.hpp>
#include <frost/value.hpp>

#include <algorithm>
#include <set>
#include <tuple>
#include <vector>

using namespace frst;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;

namespace
{

Map random_module()
{
    Stdlib_Registry_Builder builder;
    register_module_random(builder);
    auto registry = std::move(builder).build();
    auto module = registry.lookup_module("std.random");
    REQUIRE(module.has_value());
    REQUIRE(module.value()->is<Map>());
    return module.value()->raw_get<Map>();
}

Function lookup_fn(const Map& mod, const std::string& name)
{
    auto key = Value::create(String{name});
    auto it = mod.find(key);
    REQUIRE(it != mod.end());
    REQUIRE(it->second->is<Function>());
    return it->second->raw_get<Function>();
}

Map lookup_submap(const Map& mod, const std::string& name)
{
    auto key = Value::create(String{name});
    auto it = mod.find(key);
    REQUIRE(it != mod.end());
    REQUIRE(it->second->is<Map>());
    return it->second->raw_get<Map>();
}

// Build a deterministic, seeded engine bundle.
Map seeded(Int seed_value)
{
    auto mod = random_module();
    auto seed_fn = lookup_fn(mod, "seed");
    auto bundle = seed_fn->call({Value::create(seed_value)});
    REQUIRE(bundle->is<Map>());
    return bundle->raw_get<Map>();
}

Value_Ptr int_v(Int n)
{
    return Value::create(n);
}

Value_Ptr float_v(Float n)
{
    return Value::create(n);
}

Value_Ptr int_array(Int low, Int high)
{
    Array out;
    for (Int i = low; i <= high; ++i)
        out.push_back(int_v(i));
    return Value::create(std::move(out));
}

Value_Ptr empty_array()
{
    return Value::create(Array{});
}

} // namespace

TEST_CASE("std.random module structure")
{
    auto mod = random_module();

    SECTION("seed is a top-level function")
    {
        REQUIRE(lookup_fn(mod, "seed"));
    }

    SECTION("rng is a default closure bundle")
    {
        auto rng = lookup_submap(mod, "rng");
        REQUIRE(lookup_fn(rng, "int"));
        REQUIRE(lookup_fn(rng, "float"));
        REQUIRE(lookup_fn(rng, "bool"));
        REQUIRE(lookup_fn(rng, "choice"));
        REQUIRE(lookup_fn(rng, "sample"));
        REQUIRE(lookup_fn(rng, "shuffle"));
    }

    SECTION("seeded engines have the same shape as rng")
    {
        auto bundle = seeded(42_f);
        REQUIRE(lookup_fn(bundle, "int"));
        REQUIRE(lookup_fn(bundle, "float"));
        REQUIRE(lookup_fn(bundle, "bool"));
        REQUIRE(lookup_fn(bundle, "choice"));
        REQUIRE(lookup_fn(bundle, "sample"));
        REQUIRE(lookup_fn(bundle, "shuffle"));
    }
}

TEST_CASE("std.random.seed")
{
    auto mod = random_module();
    auto seed_fn = lookup_fn(mod, "seed");

    SECTION("Returns a Map (closure bundle)")
    {
        auto bundle = seed_fn->call({int_v(42_f)});
        CHECK(bundle->is<Map>());
    }

    SECTION("Type checking: requires Int")
    {
        CHECK_THROWS_WITH(seed_fn->call({Value::create("not an int"s)}),
                          ContainsSubstring("Int"));
        CHECK_THROWS_WITH(seed_fn->call({float_v(1.5)}),
                          ContainsSubstring("Int"));
        CHECK_THROWS_WITH(seed_fn->call({Value::null()}),
                          ContainsSubstring("Int"));
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(seed_fn->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(seed_fn->call({int_v(1_f), int_v(2_f)}),
                          ContainsSubstring("too many arguments"));
    }

    SECTION("seed(0) produces a usable bundle")
    {
        auto bundle = seed_fn->call({int_v(0_f)});
        REQUIRE(bundle->is<Map>());
        auto int_fn = lookup_fn(bundle->raw_get<Map>(), "int");
        // Should not crash and should return a value in range
        auto v = int_fn->call({int_v(0_f), int_v(100_f)});
        REQUIRE(v->is<Int>());
        auto n = v->raw_get<Int>();
        CHECK(n >= 0);
        CHECK(n <= 100);
    }

    SECTION("Negative seed wraps via static_cast and produces a usable bundle")
    {
        // Frost Int is signed int64; mt19937_64 takes unsigned. The driver
        // intentionally wraps via static_cast so that any Int value is a valid
        // seed. This test exists to lock that intent in.
        auto bundle = seed_fn->call({int_v(-1_f)});
        REQUIRE(bundle->is<Map>());
        auto int_fn = lookup_fn(bundle->raw_get<Map>(), "int");
        auto v = int_fn->call({int_v(0_f), int_v(100_f)});
        REQUIRE(v->is<Int>());
        auto n = v->raw_get<Int>();
        CHECK(n >= 0);
        CHECK(n <= 100);
    }
}

TEST_CASE("std.random rng.int")
{
    auto bundle = seeded(42_f);
    auto int_fn = lookup_fn(bundle, "int");

    SECTION("Returns Int")
    {
        auto v = int_fn->call({int_v(1_f), int_v(10_f)});
        CHECK(v->is<Int>());
    }

    SECTION("Result is within bounds [low, high]")
    {
        for (int i = 0; i < 1000; ++i)
        {
            auto v = int_fn->call({int_v(1_f), int_v(10_f)});
            REQUIRE(v->is<Int>());
            auto n = v->raw_get<Int>();
            CHECK(n >= 1);
            CHECK(n <= 10);
        }
    }

    SECTION("All values in range are produced over many samples")
    {
        std::set<Int> seen;
        for (int i = 0; i < 1000; ++i)
        {
            auto v = int_fn->call({int_v(0_f), int_v(9_f)});
            seen.insert(v->raw_get<Int>());
        }
        CHECK(seen.size() == 10);
    }

    SECTION("Degenerate range (low == high) always returns that value")
    {
        for (int i = 0; i < 100; ++i)
        {
            auto v = int_fn->call({int_v(42_f), int_v(42_f)});
            REQUIRE(v->is<Int>());
            CHECK(v->raw_get<Int>() == 42);
        }
    }

    SECTION("Negative ranges are handled")
    {
        for (int i = 0; i < 100; ++i)
        {
            auto v = int_fn->call({int_v(-5_f), int_v(-1_f)});
            REQUIRE(v->is<Int>());
            auto n = v->raw_get<Int>();
            CHECK(n >= -5);
            CHECK(n <= -1);
        }
    }

    SECTION("Zero-spanning ranges are handled")
    {
        std::set<Int> seen;
        for (int i = 0; i < 1000; ++i)
        {
            auto v = int_fn->call({int_v(-5_f), int_v(5_f)});
            REQUIRE(v->is<Int>());
            auto n = v->raw_get<Int>();
            CHECK(n >= -5);
            CHECK(n <= 5);
            seen.insert(n);
        }
        // Both negative and positive values, plus zero, should appear.
        CHECK(seen.contains(0));
        CHECK(std::ranges::any_of(seen, [](Int n) { return n < 0; }));
        CHECK(std::ranges::any_of(seen, [](Int n) { return n > 0; }));
    }

    SECTION("low > high throws")
    {
        CHECK_THROWS_WITH(int_fn->call({int_v(10_f), int_v(1_f)}),
                          ContainsSubstring("low value must not be greater"));
    }

    SECTION("Type checking")
    {
        CHECK_THROWS_WITH(int_fn->call({float_v(1.5), int_v(10_f)}),
                          ContainsSubstring("Int"));
        CHECK_THROWS_WITH(int_fn->call({int_v(1_f), float_v(2.5)}),
                          ContainsSubstring("Int"));
        CHECK_THROWS_WITH(int_fn->call({Value::create("a"s), int_v(1_f)}),
                          ContainsSubstring("Int"));
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(int_fn->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(int_fn->call({int_v(1_f)}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(
            int_fn->call({int_v(1_f), int_v(2_f), int_v(3_f)}),
            ContainsSubstring("too many arguments"));
    }
}

TEST_CASE("std.random rng.float")
{
    auto bundle = seeded(42_f);
    auto float_fn = lookup_fn(bundle, "float");

    SECTION("Returns Float")
    {
        auto v = float_fn->call({float_v(0.0), float_v(1.0)});
        CHECK(v->is<Float>());
    }

    SECTION("Result is within bounds [low, high]")
    {
        for (int i = 0; i < 1000; ++i)
        {
            auto v = float_fn->call({float_v(0.0), float_v(1.0)});
            REQUIRE(v->is<Float>());
            auto x = v->raw_get<Float>();
            CHECK(x >= 0.0);
            CHECK(x <= 1.0);
        }
    }

    SECTION("Mean of many samples is approximately the midpoint")
    {
        Float sum = 0.0;
        constexpr int N = 10000;
        for (int i = 0; i < N; ++i)
        {
            auto v = float_fn->call({float_v(0.0), float_v(1.0)});
            sum += v->raw_get<Float>();
        }
        Float mean = sum / N;
        // 4σ ≈ 0.023; 0.05 is a comfortable margin
        CHECK(mean > 0.45);
        CHECK(mean < 0.55);
    }

    SECTION("Negative ranges are handled")
    {
        for (int i = 0; i < 100; ++i)
        {
            auto v = float_fn->call({float_v(-2.0), float_v(-1.0)});
            REQUIRE(v->is<Float>());
            auto x = v->raw_get<Float>();
            CHECK(x >= -2.0);
            CHECK(x <= -1.0);
        }
    }

    SECTION("Zero-spanning ranges are handled")
    {
        bool saw_negative = false;
        bool saw_positive = false;
        for (int i = 0; i < 1000; ++i)
        {
            auto v = float_fn->call({float_v(-1.0), float_v(1.0)});
            REQUIRE(v->is<Float>());
            auto x = v->raw_get<Float>();
            CHECK(x >= -1.0);
            CHECK(x <= 1.0);
            if (x < 0.0)
                saw_negative = true;
            if (x > 0.0)
                saw_positive = true;
        }
        CHECK(saw_negative);
        CHECK(saw_positive);
    }

    SECTION("Degenerate range (low == high) returns that value")
    {
        // Degenerate distributions are accepted (lo > hi is the only rejection).
        // libstdc++'s uniform_real_distribution(a, a) returns a; this test locks
        // that behavior in.
        for (int i = 0; i < 100; ++i)
        {
            auto v = float_fn->call({float_v(5.0), float_v(5.0)});
            REQUIRE(v->is<Float>());
            CHECK(v->raw_get<Float>() == 5.0);
        }
    }

    SECTION("low > high throws")
    {
        CHECK_THROWS_WITH(float_fn->call({float_v(10.0), float_v(1.0)}),
                          ContainsSubstring("low value must not be greater"));
    }

    SECTION("Type checking")
    {
        CHECK_THROWS_WITH(float_fn->call({int_v(1_f), float_v(2.0)}),
                          ContainsSubstring("Float"));
        CHECK_THROWS_WITH(float_fn->call({float_v(1.0), int_v(2_f)}),
                          ContainsSubstring("Float"));
        CHECK_THROWS_WITH(float_fn->call({Value::create("a"s), float_v(1.0)}),
                          ContainsSubstring("Float"));
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(float_fn->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(float_fn->call({float_v(1.0)}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(
            float_fn->call({float_v(1.0), float_v(2.0), float_v(3.0)}),
            ContainsSubstring("too many arguments"));
    }
}

TEST_CASE("std.random rng.bool")
{
    auto bundle = seeded(42_f);
    auto bool_fn = lookup_fn(bundle, "bool");

    SECTION("Nullary returns Bool")
    {
        auto v = bool_fn->call({});
        CHECK(v->is<Bool>());
    }

    SECTION("With probability returns Bool")
    {
        auto v = bool_fn->call({float_v(0.5)});
        CHECK(v->is<Bool>());
    }

    SECTION("prob == 0.0 always returns false")
    {
        for (int i = 0; i < 100; ++i)
        {
            auto v = bool_fn->call({float_v(0.0)});
            REQUIRE(v->is<Bool>());
            CHECK(v->raw_get<Bool>() == false);
        }
    }

    SECTION("prob == 1.0 always returns true")
    {
        for (int i = 0; i < 100; ++i)
        {
            auto v = bool_fn->call({float_v(1.0)});
            REQUIRE(v->is<Bool>());
            CHECK(v->raw_get<Bool>() == true);
        }
    }

    SECTION("Fair coin produces approximately 50% true")
    {
        int trues = 0;
        constexpr int N = 10000;
        for (int i = 0; i < N; ++i)
        {
            auto v = bool_fn->call({});
            if (v->raw_get<Bool>())
                ++trues;
        }
        Float ratio = static_cast<Float>(trues) / N;
        CHECK(ratio > 0.47);
        CHECK(ratio < 0.53);
    }

    SECTION("Biased coin (prob = 0.7) produces approximately 70% true")
    {
        int trues = 0;
        constexpr int N = 10000;
        for (int i = 0; i < N; ++i)
        {
            auto v = bool_fn->call({float_v(0.7)});
            if (v->raw_get<Bool>())
                ++trues;
        }
        Float ratio = static_cast<Float>(trues) / N;
        CHECK(ratio > 0.67);
        CHECK(ratio < 0.73);
    }

    SECTION("Probability above 1.0 throws")
    {
        CHECK_THROWS_WITH(bool_fn->call({float_v(1.5)}),
                          ContainsSubstring("probability"));
    }

    SECTION("Probability below 0.0 throws")
    {
        CHECK_THROWS_WITH(bool_fn->call({float_v(-0.1)}),
                          ContainsSubstring("probability"));
    }

    SECTION("Type checking")
    {
        CHECK_THROWS_WITH(bool_fn->call({int_v(1_f)}),
                          ContainsSubstring("Float"));
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(bool_fn->call({float_v(0.5), float_v(0.7)}),
                          ContainsSubstring("too many arguments"));
    }
}

TEST_CASE("std.random rng.choice")
{
    auto bundle = seeded(42_f);
    auto choice_fn = lookup_fn(bundle, "choice");

    SECTION("Returns an element from the input array")
    {
        auto v = choice_fn->call({int_array(1_f, 5_f)});
        REQUIRE(v->is<Int>());
        auto n = v->raw_get<Int>();
        CHECK(n >= 1);
        CHECK(n <= 5);
    }

    SECTION("Single-element array always returns that element")
    {
        auto arr = Value::create(Array{int_v(99_f)});
        for (int i = 0; i < 50; ++i)
        {
            auto v = choice_fn->call({arr});
            REQUIRE(v->is<Int>());
            CHECK(v->raw_get<Int>() == 99);
        }
    }

    SECTION("Empty array throws")
    {
        CHECK_THROWS_WITH(choice_fn->call({empty_array()}),
                          ContainsSubstring("empty"));
    }

    SECTION("All elements are reachable over many samples")
    {
        auto arr = int_array(0_f, 9_f);
        std::set<Int> seen;
        for (int i = 0; i < 1000; ++i)
        {
            auto v = choice_fn->call({arr});
            seen.insert(v->raw_get<Int>());
        }
        CHECK(seen.size() == 10);
    }

    SECTION("Heterogeneous arrays work")
    {
        // choice doesn't care about element types -- any Frost value works,
        // including Null. Each result must be one of the input elements
        // (pointer-identity match, since the bundle preserves Value_Ptr).
        Array elements{
            int_v(1_f),
            float_v(2.5),
            Value::create("three"s),
            Value::null(),
        };
        auto arr = Value::create(Array{elements});
        for (int i = 0; i < 50; ++i)
        {
            auto v = choice_fn->call({arr});
            CHECK(std::ranges::any_of(
                elements, [&](const auto& e) { return e == v; }));
        }
    }

    SECTION("Type checking")
    {
        CHECK_THROWS_WITH(choice_fn->call({int_v(1_f)}),
                          ContainsSubstring("Array"));
        CHECK_THROWS_WITH(choice_fn->call({Value::create("hello"s)}),
                          ContainsSubstring("Array"));
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(choice_fn->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(choice_fn->call({empty_array(), empty_array()}),
                          ContainsSubstring("too many arguments"));
    }
}

TEST_CASE("std.random rng.sample")
{
    auto bundle = seeded(42_f);
    auto sample_fn = lookup_fn(bundle, "sample");

    SECTION("Returns Array of length n")
    {
        auto v = sample_fn->call({int_array(1_f, 10_f), int_v(3_f)});
        REQUIRE(v->is<Array>());
        CHECK(v->raw_get<Array>().size() == 3);
    }

    SECTION("All sampled elements come from the input")
    {
        auto v = sample_fn->call({int_array(1_f, 10_f), int_v(5_f)});
        REQUIRE(v->is<Array>());
        for (const auto& elem : v->raw_get<Array>())
        {
            REQUIRE(elem->is<Int>());
            auto n = elem->raw_get<Int>();
            CHECK(n >= 1);
            CHECK(n <= 10);
        }
    }

    SECTION("Sampled elements are distinct")
    {
        auto v = sample_fn->call({int_array(0_f, 99_f), int_v(20_f)});
        REQUIRE(v->is<Array>());
        std::set<Int> seen;
        for (const auto& elem : v->raw_get<Array>())
            seen.insert(elem->raw_get<Int>());
        CHECK(seen.size() == 20);
    }

    SECTION("n == 0 returns an empty array")
    {
        auto v = sample_fn->call({int_array(1_f, 10_f), int_v(0_f)});
        REQUIRE(v->is<Array>());
        CHECK(v->raw_get<Array>().empty());
    }

    SECTION("n == len(arr) returns a permutation of the input")
    {
        auto v = sample_fn->call({int_array(1_f, 10_f), int_v(10_f)});
        REQUIRE(v->is<Array>());
        const auto& result = v->raw_get<Array>();
        REQUIRE(result.size() == 10);
        std::set<Int> seen;
        for (const auto& elem : result)
            seen.insert(elem->raw_get<Int>());
        CHECK(seen.size() == 10);
        CHECK(*seen.begin() == 1);
        CHECK(*seen.rbegin() == 10);
    }

    SECTION("Heterogeneous arrays work")
    {
        Array elements{
            int_v(1_f),
            float_v(2.5),
            Value::create("three"s),
            Value::null(),
        };
        auto arr = Value::create(Array{elements});
        auto v = sample_fn->call({arr, int_v(2_f)});
        REQUIRE(v->is<Array>());
        const auto& result = v->raw_get<Array>();
        REQUIRE(result.size() == 2);
        // Each sampled element must be one of the input elements.
        for (const auto& elem : result)
            CHECK(std::ranges::any_of(
                elements, [&](const auto& e) { return e == elem; }));
    }

    SECTION("Negative n throws")
    {
        CHECK_THROWS_WITH(
            sample_fn->call({int_array(1_f, 10_f), int_v(-1_f)}),
            ContainsSubstring("negative"));
    }

    SECTION("n > len(arr) throws")
    {
        CHECK_THROWS_WITH(
            sample_fn->call({int_array(1_f, 5_f), int_v(10_f)}),
            ContainsSubstring("greater than the input size"));
    }

    SECTION("Type checking")
    {
        CHECK_THROWS_WITH(sample_fn->call({int_v(1_f), int_v(1_f)}),
                          ContainsSubstring("Array"));
        CHECK_THROWS_WITH(
            sample_fn->call({int_array(1_f, 5_f), float_v(1.5)}),
            ContainsSubstring("Int"));
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(sample_fn->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(sample_fn->call({int_array(1_f, 5_f)}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(
            sample_fn->call({int_array(1_f, 5_f), int_v(1_f), int_v(2_f)}),
            ContainsSubstring("too many arguments"));
    }
}

TEST_CASE("std.random rng.shuffle")
{
    auto bundle = seeded(42_f);
    auto shuffle_fn = lookup_fn(bundle, "shuffle");

    SECTION("Returns Array of same length")
    {
        auto v = shuffle_fn->call({int_array(1_f, 10_f)});
        REQUIRE(v->is<Array>());
        CHECK(v->raw_get<Array>().size() == 10);
    }

    SECTION("Result is a permutation (same multiset of elements)")
    {
        auto v = shuffle_fn->call({int_array(1_f, 10_f)});
        REQUIRE(v->is<Array>());
        std::set<Int> seen;
        for (const auto& elem : v->raw_get<Array>())
            seen.insert(elem->raw_get<Int>());
        CHECK(seen.size() == 10);
        CHECK(*seen.begin() == 1);
        CHECK(*seen.rbegin() == 10);
    }

    SECTION("Empty array -> empty array")
    {
        auto v = shuffle_fn->call({empty_array()});
        REQUIRE(v->is<Array>());
        CHECK(v->raw_get<Array>().empty());
    }

    SECTION("Single-element array is unchanged")
    {
        auto arr = Value::create(Array{int_v(42_f)});
        auto v = shuffle_fn->call({arr});
        REQUIRE(v->is<Array>());
        REQUIRE(v->raw_get<Array>().size() == 1);
        CHECK(v->raw_get<Array>()[0]->raw_get<Int>() == 42);
    }

    SECTION("Does not modify the input value")
    {
        auto input = int_array(1_f, 5_f);
        shuffle_fn->call({input});
        // The original Value_Ptr's underlying Array should be untouched.
        const auto& orig = input->raw_get<Array>();
        REQUIRE(orig.size() == 5);
        for (Int i = 0; i < 5; ++i)
            CHECK(orig[i]->raw_get<Int>() == i + 1);
    }

    SECTION("Over many trials, all permutations of a small array appear")
    {
        // 3! = 6 permutations; with 1000 shuffles the probability of missing
        // any specific permutation is (5/6)^1000 ~ 5e-80. Safe.
        std::set<std::tuple<Int, Int, Int>> perms;
        for (int i = 0; i < 1000; ++i)
        {
            auto v = shuffle_fn->call({int_array(0_f, 2_f)});
            const auto& result = v->raw_get<Array>();
            perms.insert({result[0]->raw_get<Int>(),
                          result[1]->raw_get<Int>(),
                          result[2]->raw_get<Int>()});
        }
        CHECK(perms.size() == 6);
    }

    SECTION("Preserves multiset on arrays with duplicates")
    {
        auto input = Value::create(Array{
            int_v(1_f), int_v(1_f), int_v(2_f), int_v(2_f),
        });
        for (int i = 0; i < 50; ++i)
        {
            auto v = shuffle_fn->call({input});
            REQUIRE(v->is<Array>());
            const auto& result = v->raw_get<Array>();
            REQUIRE(result.size() == 4);
            int ones = 0;
            int twos = 0;
            for (const auto& elem : result)
            {
                if (elem->raw_get<Int>() == 1)
                    ++ones;
                else if (elem->raw_get<Int>() == 2)
                    ++twos;
            }
            CHECK(ones == 2);
            CHECK(twos == 2);
        }
    }

    SECTION("Heterogeneous arrays work")
    {
        Array elements{
            int_v(1_f),
            float_v(2.5),
            Value::create("three"s),
            Value::null(),
        };
        auto arr = Value::create(Array{elements});
        auto v = shuffle_fn->call({arr});
        REQUIRE(v->is<Array>());
        const auto& result = v->raw_get<Array>();
        REQUIRE(result.size() == 4);
        // Result must be a permutation: same multiset of element pointers.
        std::multiset<Value_Ptr> input_set(elements.begin(), elements.end());
        std::multiset<Value_Ptr> result_set(result.begin(), result.end());
        CHECK(input_set == result_set);
    }

    SECTION("Type checking")
    {
        CHECK_THROWS_WITH(shuffle_fn->call({int_v(1_f)}),
                          ContainsSubstring("Array"));
        CHECK_THROWS_WITH(shuffle_fn->call({Value::create("hello"s)}),
                          ContainsSubstring("Array"));
    }

    SECTION("Arity")
    {
        CHECK_THROWS_WITH(shuffle_fn->call({}),
                          ContainsSubstring("insufficient arguments"));
        CHECK_THROWS_WITH(shuffle_fn->call({empty_array(), empty_array()}),
                          ContainsSubstring("too many arguments"));
    }
}

TEST_CASE("std.random reproducibility")
{
    SECTION("Same seed produces the same sequence of ints")
    {
        auto a = seeded(12345_f);
        auto b = seeded(12345_f);
        auto a_int = lookup_fn(a, "int");
        auto b_int = lookup_fn(b, "int");
        for (int i = 0; i < 100; ++i)
        {
            auto va = a_int->call({int_v(0_f), int_v(1000000_f)});
            auto vb = b_int->call({int_v(0_f), int_v(1000000_f)});
            REQUIRE(va->raw_get<Int>() == vb->raw_get<Int>());
        }
    }

    SECTION("Same seed produces the same sequence across mixed operations")
    {
        auto a = seeded(7_f);
        auto b = seeded(7_f);
        auto a_int = lookup_fn(a, "int");
        auto a_float = lookup_fn(a, "float");
        auto a_bool = lookup_fn(a, "bool");
        auto b_int = lookup_fn(b, "int");
        auto b_float = lookup_fn(b, "float");
        auto b_bool = lookup_fn(b, "bool");
        for (int i = 0; i < 50; ++i)
        {
            CHECK(a_int->call({int_v(0_f), int_v(100_f)})->raw_get<Int>()
                  == b_int->call({int_v(0_f), int_v(100_f)})->raw_get<Int>());
            CHECK(a_float->call({float_v(0.0), float_v(1.0)})->raw_get<Float>()
                  == b_float->call({float_v(0.0), float_v(1.0)})
                         ->raw_get<Float>());
            CHECK(a_bool->call({})->raw_get<Bool>()
                  == b_bool->call({})->raw_get<Bool>());
        }
    }

    SECTION("Different seeds produce different sequences")
    {
        auto a = seeded(1_f);
        auto b = seeded(2_f);
        auto a_int = lookup_fn(a, "int");
        auto b_int = lookup_fn(b, "int");
        // Collect 10 draws from each; they should not all match.
        bool any_diff = false;
        for (int i = 0; i < 10; ++i)
        {
            auto va = a_int->call({int_v(0_f), int_v(1000000_f)});
            auto vb = b_int->call({int_v(0_f), int_v(1000000_f)});
            if (va->raw_get<Int>() != vb->raw_get<Int>())
            {
                any_diff = true;
                break;
            }
        }
        CHECK(any_diff);
    }

    SECTION("Independent engines have independent state")
    {
        // Interleaving calls to two engines should not affect either engine's
        // sequence relative to a non-interleaved baseline.
        auto baseline = seeded(99_f);
        auto baseline_int = lookup_fn(baseline, "int");
        std::vector<Int> baseline_seq;
        for (int i = 0; i < 20; ++i)
        {
            baseline_seq.push_back(
                baseline_int->call({int_v(0_f), int_v(1000_f)})
                    ->raw_get<Int>());
        }

        auto a = seeded(99_f);
        auto b = seeded(123_f); // unrelated
        auto a_int = lookup_fn(a, "int");
        auto b_int = lookup_fn(b, "int");
        std::vector<Int> interleaved_seq;
        for (int i = 0; i < 20; ++i)
        {
            interleaved_seq.push_back(
                a_int->call({int_v(0_f), int_v(1000_f)})->raw_get<Int>());
            // Pull from b too; should not affect a's sequence.
            (void) b_int->call({int_v(0_f), int_v(1000_f)});
        }

        CHECK(baseline_seq == interleaved_seq);
    }

    SECTION("Same seed produces the same shuffle output")
    {
        auto a = seeded(2024_f);
        auto b = seeded(2024_f);
        auto a_shuffle = lookup_fn(a, "shuffle");
        auto b_shuffle = lookup_fn(b, "shuffle");
        for (int i = 0; i < 20; ++i)
        {
            auto va = a_shuffle->call({int_array(0_f, 9_f)});
            auto vb = b_shuffle->call({int_array(0_f, 9_f)});
            const auto& arr_a = va->raw_get<Array>();
            const auto& arr_b = vb->raw_get<Array>();
            REQUIRE(arr_a.size() == arr_b.size());
            for (std::size_t j = 0; j < arr_a.size(); ++j)
                CHECK(arr_a[j]->raw_get<Int>() == arr_b[j]->raw_get<Int>());
        }
    }

    SECTION("Same seed produces the same sample output")
    {
        auto a = seeded(2024_f);
        auto b = seeded(2024_f);
        auto a_sample = lookup_fn(a, "sample");
        auto b_sample = lookup_fn(b, "sample");
        for (int i = 0; i < 20; ++i)
        {
            auto va = a_sample->call({int_array(0_f, 99_f), int_v(10_f)});
            auto vb = b_sample->call({int_array(0_f, 99_f), int_v(10_f)});
            const auto& arr_a = va->raw_get<Array>();
            const auto& arr_b = vb->raw_get<Array>();
            REQUIRE(arr_a.size() == arr_b.size());
            for (std::size_t j = 0; j < arr_a.size(); ++j)
                CHECK(arr_a[j]->raw_get<Int>() == arr_b[j]->raw_get<Int>());
        }
    }

    SECTION("Same seed produces the same choice output")
    {
        auto a = seeded(2024_f);
        auto b = seeded(2024_f);
        auto a_choice = lookup_fn(a, "choice");
        auto b_choice = lookup_fn(b, "choice");
        for (int i = 0; i < 100; ++i)
        {
            auto va = a_choice->call({int_array(0_f, 99_f)});
            auto vb = b_choice->call({int_array(0_f, 99_f)});
            CHECK(va->raw_get<Int>() == vb->raw_get<Int>());
        }
    }
}

TEST_CASE("std.random bundle aliasing")
{
    SECTION("Functions from the same bundle share engine state")
    {
        // Drawing an int then a float from the same bundle should match
        // drawing an int then a float from a freshly seeded equivalent bundle.
        auto bundle = seeded(42_f);
        auto int_fn = lookup_fn(bundle, "int");
        auto float_fn = lookup_fn(bundle, "float");
        auto i1 = int_fn->call({int_v(0_f), int_v(1000000_f)})->raw_get<Int>();
        auto f1 = float_fn->call({float_v(0.0), float_v(1.0)})->raw_get<Float>();

        auto bundle2 = seeded(42_f);
        auto int_fn_2 = lookup_fn(bundle2, "int");
        auto float_fn_2 = lookup_fn(bundle2, "float");
        auto i2 =
            int_fn_2->call({int_v(0_f), int_v(1000000_f)})->raw_get<Int>();
        auto f2 = float_fn_2->call({float_v(0.0), float_v(1.0)})
                      ->raw_get<Float>();

        CHECK(i1 == i2);
        CHECK(f1 == f2);
    }

    SECTION("Multiple lookups of the same function share engine state")
    {
        // Looking up `int` twice on the same bundle should give two handles
        // that advance the same underlying engine.
        auto bundle = seeded(7_f);
        auto fn_a = lookup_fn(bundle, "int");
        auto fn_b = lookup_fn(bundle, "int");
        auto v1 = fn_a->call({int_v(0_f), int_v(1000000_f)})->raw_get<Int>();
        auto v2 = fn_b->call({int_v(0_f), int_v(1000000_f)})->raw_get<Int>();

        // Compare with two consecutive draws via a single handle from a fresh
        // bundle with the same seed.
        auto bundle2 = seeded(7_f);
        auto fn = lookup_fn(bundle2, "int");
        auto u1 = fn->call({int_v(0_f), int_v(1000000_f)})->raw_get<Int>();
        auto u2 = fn->call({int_v(0_f), int_v(1000000_f)})->raw_get<Int>();

        CHECK(v1 == u1);
        CHECK(v2 == u2);
    }

    SECTION("Copying the bundle Map does not fork the engine")
    {
        // Frost users may pass a seeded bundle around (`def b = a`); both
        // references should observe the same engine state.
        auto bundle_a = seeded(123_f);
        auto bundle_b = bundle_a; // shallow copy of the Map
        auto fn_a = lookup_fn(bundle_a, "int");
        auto fn_b = lookup_fn(bundle_b, "int");
        auto v_a = fn_a->call({int_v(0_f), int_v(1000000_f)})->raw_get<Int>();
        auto v_b = fn_b->call({int_v(0_f), int_v(1000000_f)})->raw_get<Int>();

        // Same as drawing twice from a fresh seeded(123) bundle.
        auto fresh = seeded(123_f);
        auto fresh_fn = lookup_fn(fresh, "int");
        auto u1 =
            fresh_fn->call({int_v(0_f), int_v(1000000_f)})->raw_get<Int>();
        auto u2 =
            fresh_fn->call({int_v(0_f), int_v(1000000_f)})->raw_get<Int>();

        CHECK(v_a == u1);
        CHECK(v_b == u2);
    }
}

TEST_CASE("std.random default rng smoke test")
{
    auto mod = random_module();
    auto rng = lookup_submap(mod, "rng");

    SECTION("int returns Int within bounds")
    {
        auto int_fn = lookup_fn(rng, "int");
        auto v = int_fn->call({int_v(1_f), int_v(10_f)});
        REQUIRE(v->is<Int>());
        auto n = v->raw_get<Int>();
        CHECK(n >= 1);
        CHECK(n <= 10);
    }

    SECTION("float returns Float within bounds")
    {
        auto float_fn = lookup_fn(rng, "float");
        auto v = float_fn->call({float_v(0.0), float_v(1.0)});
        REQUIRE(v->is<Float>());
        auto x = v->raw_get<Float>();
        CHECK(x >= 0.0);
        CHECK(x <= 1.0);
    }

    SECTION("bool returns Bool")
    {
        auto bool_fn = lookup_fn(rng, "bool");
        auto v = bool_fn->call({});
        CHECK(v->is<Bool>());
    }

    SECTION("choice returns an element of the input")
    {
        auto choice_fn = lookup_fn(rng, "choice");
        auto v = choice_fn->call({int_array(1_f, 5_f)});
        REQUIRE(v->is<Int>());
        auto n = v->raw_get<Int>();
        CHECK(n >= 1);
        CHECK(n <= 5);
    }

    SECTION("sample returns an Array of the requested size")
    {
        auto sample_fn = lookup_fn(rng, "sample");
        auto v = sample_fn->call({int_array(1_f, 10_f), int_v(3_f)});
        REQUIRE(v->is<Array>());
        CHECK(v->raw_get<Array>().size() == 3);
    }

    SECTION("shuffle returns an Array of the same length")
    {
        auto shuffle_fn = lookup_fn(rng, "shuffle");
        auto v = shuffle_fn->call({int_array(1_f, 5_f)});
        REQUIRE(v->is<Array>());
        CHECK(v->raw_get<Array>().size() == 5);
    }
}
