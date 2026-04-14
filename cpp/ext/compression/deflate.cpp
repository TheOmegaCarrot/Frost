#include "zlib-common.hpp"

#include <frost/builtins-common.hpp>

#include <zlib.h>

namespace frst::compression::deflate
{

BUILTIN(compress)
{
    return zlib_common::compress("deflate.compress", args, -MAX_WBITS);
}

BUILTIN(decompress)
{
    return zlib_common::decompress("deflate.decompress", args, -MAX_WBITS);
}

} // namespace frst::compression::deflate
