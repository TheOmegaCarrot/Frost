#include <frost/ext.hpp>

namespace frst
{

#define DO_REGISTER_EXTENSION(name)                                            \
    void register_module_##name(Stdlib_Registry_Builder&);                     \
    register_module_##name(builder);

void register_extensions(Stdlib_Registry_Builder& builder)
{
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wvexing-parse"
    // The "vexing parse" is _actually_ intended here

#ifdef FROST_ENABLE_HASH
    DO_REGISTER_EXTENSION(hash);
#endif

#ifdef FROST_ENABLE_HTTP
    DO_REGISTER_EXTENSION(http);
#endif

#ifdef FROST_ENABLE_SQLITE
    DO_REGISTER_EXTENSION(sqlite);
#endif

#ifdef FROST_ENABLE_COMPRESSION
    DO_REGISTER_EXTENSION(compression);
#endif

#ifdef FROST_ENABLE_MSGPACK
    DO_REGISTER_EXTENSION(msgpack);
#endif

#ifdef FROST_ENABLE_TOML
    DO_REGISTER_EXTENSION(toml);
#endif

#ifdef FROST_ENABLE_UNSAFE
    DO_REGISTER_EXTENSION(unsafe);
#endif

#ifdef FROST_ENABLE_EXAMPLE
    DO_REGISTER_EXTENSION(example);
#endif

#ifdef FROST_ENABLE_CSV
    DO_REGISTER_EXTENSION(csv);
#endif

#pragma GCC diagnostic pop
}

} // namespace frst
