#include "builtins-common.hpp"
#include "frost/builtin.hpp"
#include "frost/value.hpp"

#include <frost/symbol-table.hpp>

#include <cmath>

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
    Value_Ptr FN(builtin_args_t args)                                          \
    {                                                                          \
        REQUIRE_ARGS(#FN, TYPES(Int, Float));                                  \
                                                                               \
        return Value::create(std::FN(args.at(0)->as<Float>().value()));        \
    }

X_UNARY_MATH_FLOAT

#undef X

#define X_BINARY_MATH_FLOAT                                                    \
    X(pow)                                                                     \
    X(min)                                                                     \
    X(max)                                                                     \
    X(atan2)

#define X(FN)                                                                  \
    Value_Ptr FN(builtin_args_t args)                                          \
    {                                                                          \
        REQUIRE_ARGS(#FN, TYPES(Int, Float), TYPES(Int, Float));               \
        return Value::create(auto{std::FN(args.at(0)->as<Float>().value(),     \
                                          args.at(1)->as<Float>().value())});  \
    }

X_BINARY_MATH_FLOAT

#undef X

Value_Ptr abs(builtin_args_t args)
{
    REQUIRE_ARGS("abs", TYPES(Int, Float));

    const auto& arg = args.at(0);

    if (auto iarg = arg->get<Int>())
        return Value::create(std::abs(iarg.value()));
    else if (auto farg = arg->get<Float>())
        return Value::create(std::abs(farg.value()));

    THROW_UNREACHABLE;
}

Value_Ptr round(builtin_args_t args)
{
    REQUIRE_ARGS("round", TYPES(Int, Float));

    return Value::create(std::lround(args.at(0)->as<Float>().value()));
}

Value_Ptr hypot(builtin_args_t args)
{
    REQUIRE_ARGS("hypot", TYPES(Int, Float), TYPES(Int, Float),
                 OPTIONAL(TYPES(Int, Float)));

    if (args.size() == 2)
    {
        return Value::create(std::hypot(args.at(0)->as<Float>().value(),
                                        args.at(1)->as<Float>().value()));
    }
    else if (args.size() == 3)
    {
        return Value::create(std::hypot(args.at(0)->as<Float>().value(),
                                        args.at(1)->as<Float>().value(),
                                        args.at(2)->as<Float>().value()));
    }

    THROW_UNREACHABLE;
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
}

} // namespace frst
