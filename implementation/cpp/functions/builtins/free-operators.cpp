#include "builtins-common.hpp"
#include "frost/builtin.hpp"

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

namespace frst
{

// X(frost_name, internal_name)
#define X_FREE_BINOP                                                           \
    X(plus, add)                                                               \
    X(minus, subtract)                                                         \
    X(times, multiply)                                                         \
    X(divide, divide)                                                          \
    X(equal, equal)                                                            \
    X(not_equal, not_equal)                                                    \
    X(less_than, less_than)                                                    \
    X(less_than_or_equal, less_than_or_equal)                                  \
    X(greater_than, greater_than)                                              \
    X(greater_than_or_equal, greater_than_or_equal)

#define X(frost_name, internal_name)                                           \
    BUILTIN(frost_name)                                                        \
    {                                                                          \
        return Value::internal_name(args.at(0), args.at(1));                   \
    }

X_FREE_BINOP

#undef X

BUILTIN(deep_equal)
{
    return Value::deep_equal(args.at(0), args.at(1));
}

void inject_free_operators(Symbol_Table& table)
{
#define X(frost_name, internal_name) INJECT(frost_name, 2, 2);

    X_FREE_BINOP

#undef X

    INJECT(deep_equal, 2, 2);
}
} // namespace frst
