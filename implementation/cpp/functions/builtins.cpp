#include "frost/builtin.hpp"
#include <frost/symbol-table.hpp>

namespace frst
{

#define X_INJECT                                                               \
    X(structure_ops)                                                           \
    X(type_checks)                                                             \
    X(type_conversions)                                                        \
    X(pack_call)                                                               \
    X(debug_helpers)                                                           \
    X(error_handling)                                                          \
    X(output)

#define X(F) void inject_##F(Symbol_Table&);

X_INJECT

#undef X

void inject_builtins(Symbol_Table& table)
{
#define X(F) inject_##F(table);

    X_INJECT

#undef X
}
} // namespace frst
