#include <frost/builtin.hpp>
#include <frost/builtins-common.hpp>
#include <frost/value.hpp>

#include <random>

namespace frst
{

namespace random
{

namespace
{

class Engine
{
  public:
    template <typename... Args>
    explicit Engine(Args&&... args)
        : rng_{std::forward<Args>(args)...}
    {
    }

  private:
    struct Unique_Engine
    {
        std::mt19937_64& rng;
        std::lock_guard<std::mutex> lock;
    };

  public:
    Unique_Engine hold()
    {
        return {rng_, std::lock_guard{mutex_}};
    }

    std::mt19937_64 rng_;
    std::mutex mutex_;
};

Value_Ptr make_random(std::shared_ptr<Engine> engine)
{
    return Value::create(
        Value::trusted,
        Map{
            {
                "int"_s,
                system_closure([engine](builtin_args_t args) {
                    REQUIRE_ARGS("rng.int", PARAM("low", TYPES(Int)),
                                 PARAM("high", TYPES(Int)));
                    auto lo = GET(0, Int);
                    auto hi = GET(1, Int);
                    if (lo > hi)
                        throw Frost_Recoverable_Error{
                            "rng.int: low value must not be greater than high "
                            "value"};
                    return Value::create(std::uniform_int_distribution(lo, hi)(
                        engine->hold().rng));
                }),
            },
            {
                "float"_s,
                system_closure([engine](builtin_args_t args) {
                    REQUIRE_ARGS("rng.float", PARAM("low", TYPES(Float)),
                                 PARAM("high", TYPES(Float)));
                    auto lo = GET(0, Float);
                    auto hi = GET(1, Float);
                    if (lo > hi)
                        throw Frost_Recoverable_Error{
                            "rng.float: low value must not be greater than "
                            "high value"};
                    return Value::create(std::uniform_real_distribution(lo, hi)(
                        engine->hold().rng));
                }),
            },
            {
                "bool"_s,
                system_closure([engine](builtin_args_t args) {
                    REQUIRE_ARGS("rng.bool",
                                 OPTIONAL(PARAM("probability", TYPES(Float))));
                    Float prob = HAS(0) ? GET(0, Float) : 0.5;

                    if (prob < 0.0 || prob > 1.0)
                        throw Frost_Recoverable_Error{
                            "rng.bool: probability must be in [0.0, 1.0]"};

                    return Value::create(
                        std::bernoulli_distribution(prob)(engine->hold().rng));
                }),
            },
            {
                "choice"_s,
                system_closure([engine](builtin_args_t args) {
                    REQUIRE_ARGS("rng.choice", TYPES(Array));
                    const auto& arr = GET(0, Array);
                    if (arr.empty())
                        throw Frost_Recoverable_Error{
                            "rng.choice: input array is empty"};

                    auto idx = std::uniform_int_distribution(
                        0uz, arr.size() - 1)(engine->hold().rng);
                    return arr.at(idx);
                }),
            },
            {
                "sample"_s,
                system_closure([engine](builtin_args_t args) {
                    REQUIRE_ARGS("rng.sample", TYPES(Array),
                                 PARAM("n", TYPES(Int)));
                    const auto& arr = GET(0, Array);
                    auto n = GET(1, Int);
                    if (n < 0)
                        throw Frost_Recoverable_Error{
                            "rng.sample: n must not be negative"};
                    if (static_cast<std::size_t>(n) > arr.size())
                        throw Frost_Recoverable_Error{
                            "rng.sample: n must not be greater than the input "
                            "size"};

                    Array out;
                    std::ranges::sample(arr, std::back_inserter(out), n,
                                        engine->hold().rng);
                    return Value::create(std::move(out));
                }),
            },
            {
                "shuffle"_s,
                system_closure([engine](builtin_args_t args) {
                    REQUIRE_ARGS("rng.shuffle", TYPES(Array));
                    Array out = GET(0, Array);
                    std::ranges::shuffle(out, engine->hold().rng);
                    return Value::create(std::move(out));
                }),
            },
        });
}

} // namespace

BUILTIN(seed)
{
    REQUIRE_ARGS("random.seed", PARAM("seed", TYPES(Int)));
    return make_random(std::make_shared<Engine>(
        static_cast<std::mt19937_64::result_type>(GET(0, Int))));
}

} // namespace random

STDLIB_MODULE(random, ENTRY(seed),
              {"rng"_s,
               make_random(std::make_shared<Engine>(std::random_device{}()))})

} // namespace frst
