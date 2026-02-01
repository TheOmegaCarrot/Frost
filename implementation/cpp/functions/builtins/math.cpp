#include <frost/builtins-common.hpp>
#include "frost/builtin.hpp"
#include "frost/value.hpp"

#include <frost/symbol-table.hpp>

#include <cmath>
#include <limits>

namespace frst
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
    X(ceil)                                                                    \
    X(floor)                                                                   \
    X(trunc)                                                                   \
    X(exp)                                                                     \
    X(exp2)                                                                    \
    X(expm1)

#define X(FN)                                                                  \
    BUILTIN(FN)                                                                \
    {                                                                          \
        REQUIRE_ARGS(#FN, TYPES(Int, Float));                                  \
                                                                               \
        return Value::create(std::FN(COERCE(0, Float)));                       \
    }

X_UNARY_MATH_FLOAT

#undef X

#define X_BINARY_MATH_FLOAT                                                    \
    X(pow)                                                                     \
    X(min)                                                                     \
    X(max)                                                                     \
    X(atan2)

#define X(FN)                                                                  \
    BUILTIN(FN)                                                                \
    {                                                                          \
        REQUIRE_ARGS(#FN, TYPES(Int, Float), TYPES(Int, Float));               \
        return Value::create(                                                  \
            auto{std::FN(COERCE(0, Float), COERCE(1, Float))});                \
    }

X_BINARY_MATH_FLOAT

#undef X

BUILTIN(abs)
{
    REQUIRE_ARGS("abs", TYPES(Int, Float));

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
    REQUIRE_ARGS("round", TYPES(Int, Float));

    return Value::create(std::lround(COERCE(0, Float)));
}

BUILTIN(hypot)
{
    REQUIRE_ARGS("hypot", TYPES(Int, Float), TYPES(Int, Float),
                 OPTIONAL(TYPES(Int, Float)));

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

BUILTIN(mod)
{
    REQUIRE_ARGS("mod", TYPES(Int), TYPES(Int));

    auto lhs = GET(0, Int);
    auto rhs = GET(1, Int);

    if (rhs == 0)
        throw Frost_Recoverable_Error{"Cannot modulus by 0"};

    if (rhs == -1 && lhs == std::numeric_limits<Int>::min())
    {
        throw Frost_Recoverable_Error{
            "Function mod cannot modulus minimum Int by -1"};
    }

    return Value::create(lhs % rhs);
}

void inject_math(Symbol_Table& table)
{

#define X(FN) INJECT(FN, 1, 1);

    X_UNARY_MATH_FLOAT

#undef X

#define X(FN) INJECT(FN, 2, 2);

    X_BINARY_MATH_FLOAT

#undef X

    INJECT(abs, 1, 1);
    INJECT(round, 1, 1);
    INJECT(hypot, 2, 3);
    INJECT(mod, 2, 2);
}

} // namespace frst
