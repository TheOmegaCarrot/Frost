#include <frost/builtins-common.hpp>

#include <bzlib.h>

#include <array>

namespace frst::compression::bz2
{

BUILTIN(compress)
{
    REQUIRE_ARGS("bz2.compress", TYPES(String),
                 OPTIONAL(PARAM("level", TYPES(Int))));

    const auto& input = GET(0, String);

    int block_size = 9;
    if (HAS(1))
    {
        block_size = static_cast<int>(GET(1, Int));
        if (block_size < 1 || block_size > 9)
            throw Frost_Recoverable_Error{
                "bz2.compress: level must be between 1 and 9"};
    }

    // bz2 worst case: input size + 1% + 600 bytes (round up to cover truncation)
    auto output_size = static_cast<unsigned int>(
        input.size() + (input.size() + 99) / 100 + 600);
    std::string output(output_size, '\0');

    int ret = BZ2_bzBuffToBuffCompress(
        output.data(), &output_size,
        const_cast<char*>(input.data()),
        static_cast<unsigned int>(input.size()),
        block_size, 0, 0);

    if (ret != BZ_OK)
        throw Frost_Recoverable_Error{
            fmt::format("bz2.compress: compression failed (error {})", ret)};

    output.resize(output_size);
    return Value::create(std::move(output));
}

BUILTIN(decompress)
{
    REQUIRE_ARGS("bz2.decompress", TYPES(String));

    const auto& input = GET(0, String);

    bz_stream stream{};
    stream.next_in = const_cast<char*>(input.data());
    stream.avail_in = static_cast<unsigned int>(input.size());

    int ret = BZ2_bzDecompressInit(&stream, 0, 0);
    if (ret != BZ_OK)
        throw Frost_Recoverable_Error{
            "bz2.decompress: failed to initialize decompression stream"};

    std::string output;
    std::array<char, 16384> buf;

    do
    {
        stream.next_out = buf.data();
        stream.avail_out = static_cast<unsigned int>(buf.size());

        ret = BZ2_bzDecompress(&stream);

        if (ret != BZ_OK && ret != BZ_STREAM_END)
        {
            BZ2_bzDecompressEnd(&stream);
            throw Frost_Recoverable_Error{fmt::format(
                "bz2.decompress: decompression failed (error {})", ret)};
        }

        output.append(buf.data(), buf.size() - stream.avail_out);
    }
    while (ret != BZ_STREAM_END);

    BZ2_bzDecompressEnd(&stream);
    return Value::create(std::move(output));
}

} // namespace frst::compression::bz2
