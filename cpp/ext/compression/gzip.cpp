#include "zlib-common.hpp"

#include <frost/builtins-common.hpp>

#include <zlib.h>

namespace frst::compression::gzip
{

BUILTIN(compress)
{
    return zlib_common::compress("gzip.compress", args, MAX_WBITS + 16);
}

BUILTIN(decompress)
{
    return zlib_common::decompress("gzip.decompress", args, MAX_WBITS + 16);
}

} // namespace frst::compression::gzip
