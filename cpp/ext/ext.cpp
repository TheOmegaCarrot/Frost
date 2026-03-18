#include <frost/ext.hpp>

namespace frst
{

#define DEFINE_EXTENSION(name)                                                 \
    Map make_extension_##name();                                               \
    table.define(#name, Value::create(Value::trusted, make_extension_##name()));

void inject_ext(Symbol_Table& table)
{
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wvexing-parse"
    // The "vexing parse" is _actually_ intended here

#ifdef FROST_ENABLE_HTTP
    DEFINE_EXTENSION(http);
#endif

#pragma GCC diagnostic pop
}

} // namespace frst
