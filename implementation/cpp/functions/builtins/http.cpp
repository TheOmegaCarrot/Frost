#include <frost/builtins-common.hpp>

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

namespace frst
{

namespace http
{
BUILTIN(request)
{
}

void inject_http(Symbol_Table& table)
{
    INJECT_MAP("http", ENTRY(request, 1, 1));
}
} // namespace http
} // namespace frst
