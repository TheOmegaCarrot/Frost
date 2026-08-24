#include "decompress-limits.hpp"

#include <frost/builtins-common.hpp>

#include <snappy-c.h>

namespace frst::compression::snappy
{

BUILTIN(compress)
{
    REQUIRE_ARGS("snappy.compress", TYPES(String));

    const auto& input = GET(0, String);

    size_t output_size = snappy_max_compressed_length(input.size());
    std::string output(output_size, '\0');

    auto status = snappy_compress(input.data(), input.size(), output.data(),
                                  &output_size);

    if (status != SNAPPY_OK)
        throw Frost_Recoverable_Error{"snappy.compress: compression failed"};

    output.resize(output_size);
    return Value::create(std::move(output));
}

BUILTIN(decompress)
{
    REQUIRE_ARGS("snappy.decompress", TYPES(String),
                 OPTIONAL(PARAM("max_size", TYPES(Int))));

    const auto& input = GET(0, String);

    size_t output_size = 0;
    if (snappy_uncompressed_length(input.data(), input.size(), &output_size)
        != SNAPPY_OK)
    {
        throw Frost_Recoverable_Error{"snappy.decompress: invalid snappy data"};
    }

    // Guard against crafted frames with a bogus uncompressed length.
    // Snappy has no streaming API, so the claimed size is allocated upfront.
    // An optional second argument overrides the cap (0 = no limit).
    size_t cap = default_max_prealloc;
    if (HAS(1))
        cap = static_cast<size_t>(GET(1, Int));

    if (cap > 0 && output_size > cap)
    {
        throw Frost_Recoverable_Error{fmt::format(
            "snappy.decompress: claimed output size {} exceeds limit of {} "
            "(pass a higher max_size to override)",
            output_size, cap)};
    }

    std::string output(output_size, '\0');

    auto status = snappy_uncompress(input.data(), input.size(), output.data(),
                                    &output_size);

    if (status != SNAPPY_OK)
        throw Frost_Recoverable_Error{
            "snappy.decompress: decompression failed"};

    output.resize(output_size);
    return Value::create(std::move(output));
}

} // namespace frst::compression::snappy
