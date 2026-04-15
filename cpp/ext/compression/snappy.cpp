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

    auto status =
        snappy_compress(input.data(), input.size(), output.data(), &output_size);

    if (status != SNAPPY_OK)
        throw Frost_Recoverable_Error{"snappy.compress: compression failed"};

    output.resize(output_size);
    return Value::create(std::move(output));
}

BUILTIN(decompress)
{
    REQUIRE_ARGS("snappy.decompress", TYPES(String));

    const auto& input = GET(0, String);

    size_t output_size = 0;
    if (snappy_uncompressed_length(input.data(), input.size(), &output_size)
        != SNAPPY_OK)
    {
        throw Frost_Recoverable_Error{
            "snappy.decompress: invalid snappy data"};
    }

    std::string output(output_size, '\0');

    auto status = snappy_uncompress(
        input.data(), input.size(), output.data(), &output_size);

    if (status != SNAPPY_OK)
        throw Frost_Recoverable_Error{
            "snappy.decompress: decompression failed"};

    output.resize(output_size);
    return Value::create(std::move(output));
}

} // namespace frst::compression::snappy
