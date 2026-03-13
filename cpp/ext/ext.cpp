#include <frost/ext.hpp>

#ifdef FROST_ENABLE_HTTP
#define OPT_EXT_HTTP X(http)
#else
#define OPT_EXT_HTTP
#endif

#define X_EXT_MODULES \
    OPT_EXT_HTTP

namespace frst
{

#define X(name) void inject_##name(Symbol_Table&);
X_EXT_MODULES
#undef X

void inject_ext(Symbol_Table& table)
{
#define X(name) inject_##name(table);
    X_EXT_MODULES
#undef X
}

} // namespace frst
