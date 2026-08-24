#include "zlib-common.hpp"

#include <frost/builtins-common.hpp>

#include <zlib.h>

namespace frst::compression::zlib
{

BUILTIN(compress)
{
    return zlib_common::compress("zlib.compress", args, MAX_WBITS);
}

BUILTIN(decompress)
{
    return zlib_common::decompress("zlib.decompress", args, MAX_WBITS);
}

} // namespace frst::compression::zlib
