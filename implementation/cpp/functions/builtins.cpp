#include "frost/builtin.hpp"
#include <frost/symbol-table.hpp>

namespace frst
{

#define X_INJECT                                                               \
    X(map_ops)                                                                 \
    X(type_checks)                                                             \
    X(pack_call)                                                               \
    X(to_string)

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
