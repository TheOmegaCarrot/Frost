#include "frost/builtin.hpp"
#include "frost/value.hpp"
#include <frost/builtins-common.hpp>

#include <frost/symbol-table.hpp>

#include <cmath>
#include <limits>

namespace frst
{

namespace math
{

#define X_UNARY_MATH_FLOAT                                                     \
    X(sqrt)                                                                    \
    X(cbrt)                                                                    \
    X(sin)                                                                     \
    X(cos)                                                                     \
    X(tan)                                                                     \
    X(asin)                                                                    \
    X(acos)                                                                    \
    X(atan)                                                                    \
    X(sinh)                                                                    \
    X(cosh)                                                                    \
    X(tanh)                                                                    \
    X(asinh)                                                                   \
    X(acosh)                                                                   \
    X(atanh)                                                                   \
    X(log)                                                                     \
    X(log1p)                                                                   \
    X(log2)                                                                    \
    X(log10)                                                                   \
    X(trunc)                                                                   \
    X(exp)                                                                     \
    X(exp2)                                                                    \
    X(expm1)

#define X(FN)                                                                  \
    BUILTIN(FN)                                                                \
    {                                                                          \
        REQUIRE_ARGS("math." #FN, TYPES(Int, Float));                          \
                                                                               \
        return Value::create(std::FN(COERCE(0, Float)));                       \
    }

X_UNARY_MATH_FLOAT

#undef X

#define X_BINARY_MATH_FLOAT                                                    \
    X(min)                                                                     \
    X(max)

#define X(FN)                                                                  \
    BUILTIN(FN)                                                                \
    {                                                                          \
        REQUIRE_ARGS("math." #FN, TYPES(Int, Float), TYPES(Int, Float));       \
        return Value::create(                                                  \
            auto{std::FN(COERCE(0, Float), COERCE(1, Float))});                \
    }

X_BINARY_MATH_FLOAT

#undef X

BUILTIN(pow)
{
    REQUIRE_ARGS("math.pow", PARAM("base", TYPES(Int, Float)),
                 PARAM("exponent", TYPES(Int, Float)));
    return Value::create(std::pow(COERCE(0, Float), COERCE(1, Float)));
}

BUILTIN(atan2)
{
    REQUIRE_ARGS("math.atan2", PARAM("y", TYPES(Int, Float)),
                 PARAM("x", TYPES(Int, Float)));
    return Value::create(std::atan2(COERCE(0, Float), COERCE(1, Float)));
}

BUILTIN(abs)
{
    REQUIRE_ARGS("math.abs", TYPES(Int, Float));

    const auto& arg = args.at(0);

    if (auto iarg = arg->get<Int>())
    {
        if (iarg.value() == std::numeric_limits<Int>::min())
        {
            throw Frost_Recoverable_Error{
                "Function abs cannot take abs of minimum Int"};
        }
        return Value::create(std::abs(iarg.value()));
    }
    else if (auto farg = arg->get<Float>())
        return Value::create(std::abs(farg.value()));

    THROW_UNREACHABLE;
}

BUILTIN(round)
{
    REQUIRE_ARGS("math.round", TYPES(Int, Float));

    const auto& arg = args.at(0);
    if (arg->is<Int>())
        return arg;

    const auto value = arg->raw_get<Float>();
    const auto rounded = std::round(static_cast<long double>(value));
    if (rounded
        < static_cast<long double>(std::numeric_limits<Int>::min())
        || rounded
        > static_cast<long double>(std::numeric_limits<Int>::max()))
    {
        throw Frost_Recoverable_Error{
            fmt::format("Value {} is out of range of Int", value)};
    }

    return Value::create(static_cast<Int>(std::llround(rounded)));
}

BUILTIN(hypot)
{
    REQUIRE_ARGS("math.hypot", PARAM("a", TYPES(Int, Float)),
                 PARAM("b", TYPES(Int, Float)),
                 OPTIONAL(PARAM("c", TYPES(Int, Float))));

    if (args.size() == 2)
    {
        return Value::create(std::hypot(COERCE(0, Float), COERCE(1, Float)));
    }
    else if (args.size() == 3)
    {
        return Value::create(
            std::hypot(COERCE(0, Float), COERCE(1, Float), COERCE(2, Float)));
    }

    THROW_UNREACHABLE;
}

BUILTIN(lerp)
{
    REQUIRE_ARGS("math.lerp", TYPES(Int, Float), TYPES(Int, Float),
                 TYPES(Int, Float));

    return Value::create(
        std::lerp(COERCE(0, Float), COERCE(1, Float), COERCE(2, Float)));
}

BUILTIN(floor)
{
    REQUIRE_ARGS("math.floor", TYPES(Int, Float));
    return Value::create(Int{std::llround(std::floor(COERCE(0, Float)))});
}

BUILTIN(ceil)
{
    REQUIRE_ARGS("math.ceil", TYPES(Int, Float));
    return Value::create(Int{std::llround(std::ceil(COERCE(0, Float)))});
}

BUILTIN(clamp)
{
    REQUIRE_ARGS("math.clamp", TYPES(Int, Float), PARAM("lo", TYPES(Int, Float)),
                 PARAM("hi", TYPES(Int, Float)));

    if (IS(0, Int) && IS(1, Int) && IS(2, Int))
    {
        return Value::create(std::clamp(GET(0, Int), GET(1, Int), GET(2, Int)));
    }

    return Value::create(
        std::clamp(COERCE(0, Float), COERCE(1, Float), COERCE(2, Float)));
}

} // namespace math

// clang-format off
STDLIB_MODULE(math,
    ENTRY(sqrt, 1),
    ENTRY(cbrt, 1),
    ENTRY(sin, 1),
    ENTRY(cos, 1),
    ENTRY(tan, 1),
    ENTRY(asin, 1),
    ENTRY(acos, 1),
    ENTRY(atan, 1),
    ENTRY(sinh, 1),
    ENTRY(cosh, 1),
    ENTRY(tanh, 1),
    ENTRY(asinh, 1),
    ENTRY(acosh, 1),
    ENTRY(atanh, 1),
    ENTRY(log, 1),
    ENTRY(log1p, 1),
    ENTRY(log2, 1),
    ENTRY(log10, 1),
    ENTRY(trunc, 1),
    ENTRY(exp, 1),
    ENTRY(exp2, 1),
    ENTRY(expm1, 1),
    ENTRY(min, 2),
    ENTRY(max, 2),
    ENTRY(pow, 2),
    ENTRY(atan2, 2),
    ENTRY(abs, 1),
    ENTRY(round, 1),
    ENTRY_R(hypot, 2, 3),
    ENTRY(lerp, 3),
    ENTRY(floor, 1),
    ENTRY(ceil, 1),
    ENTRY(clamp, 3),
    {"nums"_s, Value::create(
        Value::trusted,
        Map{
            {"pi"_s, Value::create(std::numbers::pi)},
            {"e"_s, Value::create(std::numbers::e)},
            {"golden_ratio"_s, Value::create(std::numbers::phi)},
            {"tau"_s, Value::create(std::numbers::pi * 2)},
            {"maxint"_s, Value::create(std::numeric_limits<Int>::max())},
            {"minint"_s, Value::create(std::numeric_limits<Int>::min())},
            {"maxfloat"_s,
             Value::create(std::numeric_limits<Float>::max())},
            {"tinyfloat"_s,
             Value::create(std::numeric_limits<Float>::min())},
            {"float_epsilon"_s,
             Value::create(std::numeric_limits<Float>::epsilon())},
            {"minfloat"_s,
             Value::create(std::numeric_limits<Float>::lowest())},
        })}
)
// clang-format on
} // namespace frst
