#include <frost/extensions-common.hpp>

#define X_COMPRESSION_ALGOS X(brotli) X(gzip) X(deflate) X(zlib)

namespace frst
{
namespace compression
{

#define X(algo)                                                                \
    namespace algo                                                             \
    {                                                                          \
    BUILTIN(compress);                                                         \
    BUILTIN(decompress);                                                       \
    }

X_COMPRESSION_ALGOS

#undef X
} // namespace compression

#define X(algo)                                                                \
    {Value::create(String{#algo}),                                             \
     Value::create(Value::trusted, Map{                                        \
                                       NS_ENTRY(algo, compress),               \
                                       NS_ENTRY(algo, decompress),             \
                                   })},

REGISTER_EXTENSION(compression, X_COMPRESSION_ALGOS);

#undef X
} // namespace frst
