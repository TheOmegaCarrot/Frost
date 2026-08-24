#ifndef FROST_COMPRESSION_ZLIB_COMMON_HPP
#define FROST_COMPRESSION_ZLIB_COMMON_HPP

#include <frost/builtin.hpp>
#include <frost/value.hpp>

#include <string_view>

namespace frst::compression::zlib_common
{

Value_Ptr compress(std::string_view fn_name, builtin_args_t args,
                   int window_bits);

Value_Ptr decompress(std::string_view fn_name, builtin_args_t args,
                     int window_bits, bool allow_concat = false);

} // namespace frst::compression::zlib_common

#endif
