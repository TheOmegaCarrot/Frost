#include <frost/ext.hpp>

namespace frst
{

#define EXTENSION(name)                                                        \
    void inject_##name(Symbol_Table&);                                         \
    inject_##name(table)

void inject_ext(Symbol_Table& table)
{
#ifdef FROST_ENABLE_HTTP
    EXTENSION(http);
#endif
}

} // namespace frst
