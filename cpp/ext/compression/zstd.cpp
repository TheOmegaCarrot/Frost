#include <frost/builtins-common.hpp>

#include <zstd.h>

namespace frst::compression::zstd
{

BUILTIN(compress)
{
    REQUIRE_ARGS("zstd.compress", TYPES(String),
                 OPTIONAL(PARAM("level", TYPES(Int))));

    const auto& input = GET(0, String);

    int level = ZSTD_defaultCLevel();
    if (HAS(1))
    {
        level = static_cast<int>(GET(1, Int));
        if (level < ZSTD_minCLevel() || level > ZSTD_maxCLevel())
            throw Frost_Recoverable_Error{fmt::format(
                "zstd.compress: level must be between {} and {}",
                ZSTD_minCLevel(), ZSTD_maxCLevel())};
    }

    std::string output(ZSTD_compressBound(input.size()), '\0');

    size_t result = ZSTD_compress(output.data(), output.size(), input.data(),
                                  input.size(), level);

    if (ZSTD_isError(result))
        throw Frost_Recoverable_Error{fmt::format(
            "zstd.compress: compression failed ({})",
            ZSTD_getErrorName(result))};

    output.resize(result);
    return Value::create(std::move(output));
}

BUILTIN(decompress)
{
    REQUIRE_ARGS("zstd.decompress", TYPES(String));

    const auto& input = GET(0, String);

    auto content_size =
        ZSTD_getFrameContentSize(input.data(), input.size());

    if (content_size == ZSTD_CONTENTSIZE_ERROR)
        throw Frost_Recoverable_Error{
            "zstd.decompress: not valid zstd data"};

    // Known size: one-shot decompress
    if (content_size != ZSTD_CONTENTSIZE_UNKNOWN)
    {
        std::string output(content_size, '\0');

        size_t result = ZSTD_decompress(output.data(), output.size(),
                                        input.data(), input.size());

        if (ZSTD_isError(result))
            throw Frost_Recoverable_Error{fmt::format(
                "zstd.decompress: decompression failed ({})",
                ZSTD_getErrorName(result))};

        output.resize(result);
        return Value::create(std::move(output));
    }

    // Unknown size: streaming decompress
    auto* dstream = ZSTD_createDStream();
    if (not dstream)
        throw Frost_Recoverable_Error{
            "zstd.decompress: failed to create decompression stream"};

    ZSTD_inBuffer in_buf{input.data(), input.size(), 0};
    std::string output;
    std::array<char, 16384> buf;

    while (in_buf.pos < in_buf.size)
    {
        ZSTD_outBuffer out_buf{buf.data(), buf.size(), 0};
        size_t ret = ZSTD_decompressStream(dstream, &out_buf, &in_buf);

        if (ZSTD_isError(ret))
        {
            ZSTD_freeDStream(dstream);
            throw Frost_Recoverable_Error{fmt::format(
                "zstd.decompress: decompression failed ({})",
                ZSTD_getErrorName(ret))};
        }

        output.append(buf.data(), out_buf.pos);
    }

    ZSTD_freeDStream(dstream);
    return Value::create(std::move(output));
}

} // namespace frst::compression::zstd
