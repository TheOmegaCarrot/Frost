#include <frost/extensions-common.hpp>

#ifdef FROST_HAVE_ZLIB
#define X_ZLIB_ALGOS X(deflate) X(gzip) X(zlib)
#else
#define X_ZLIB_ALGOS
#endif

#ifdef FROST_HAVE_BZ2
#define X_BZ2_ALGOS X(bz2)
#else
#define X_BZ2_ALGOS
#endif

#ifdef FROST_HAVE_XZ
#define X_XZ_ALGOS X(xz)
#else
#define X_XZ_ALGOS
#endif

#ifdef FROST_HAVE_LZ4
#define X_LZ4_ALGOS X(lz4)
#else
#define X_LZ4_ALGOS
#endif

#ifdef FROST_HAVE_BROTLI
#define X_BROTLI_ALGOS X(brotli)
#else
#define X_BROTLI_ALGOS
#endif

#ifdef FROST_HAVE_SNAPPY
#define X_SNAPPY_ALGOS X(snappy)
#else
#define X_SNAPPY_ALGOS
#endif

#ifdef FROST_HAVE_ZSTD
#define X_ZSTD_ALGOS X(zstd)
#else
#define X_ZSTD_ALGOS
#endif

#define X_COMPRESSION_ALGOS X_ZLIB_ALGOS X_BZ2_ALGOS X_XZ_ALGOS X_LZ4_ALGOS X_BROTLI_ALGOS X_SNAPPY_ALGOS X_ZSTD_ALGOS

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
