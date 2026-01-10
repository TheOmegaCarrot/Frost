#include "frost/builtin.hpp"
#include <frost/symbol-table.hpp>

namespace frst
{
void inject_map_ops(Symbol_Table&);
void inject_type_checks(Symbol_Table&);

void inject_builtins(Symbol_Table& table)
{
    inject_map_ops(table);
    inject_type_checks(table);
}
} // namespace frst
