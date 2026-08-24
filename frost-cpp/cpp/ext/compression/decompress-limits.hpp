#ifndef FROST_COMPRESSION_DECOMPRESS_LIMITS_HPP
#define FROST_COMPRESSION_DECOMPRESS_LIMITS_HPP

#include <cstddef>

namespace frst::compression
{

// Maximum size for a single upfront allocation based on an untrusted frame
// header. Decompressors with streaming fallbacks (zstd) use this as the
// threshold to switch from one-shot to streaming. Decompressors without
// streaming (snappy) reject inputs that claim a larger output unless the
// caller provides an explicit override.
inline constexpr std::size_t default_max_prealloc = 256 * 1024 * 1024;

} // namespace frst::compression

#endif
