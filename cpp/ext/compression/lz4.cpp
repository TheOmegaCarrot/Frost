#include <frost/builtins-common.hpp>

#include <lz4frame.h>

#include <array>

namespace frst::compression::lz4
{

BUILTIN(compress)
{
    REQUIRE_ARGS("lz4.compress", TYPES(String),
                 OPTIONAL(PARAM("level", TYPES(Int))));

    const auto& input = GET(0, String);

    LZ4F_preferences_t prefs{};
    prefs.frameInfo.contentSize = input.size();

    if (HAS(1))
    {
        prefs.compressionLevel = static_cast<int>(GET(1, Int));
        if (prefs.compressionLevel > LZ4F_compressionLevel_max())
            throw Frost_Recoverable_Error{fmt::format(
                "lz4.compress: level must be at most {}",
                LZ4F_compressionLevel_max())};
    }

    size_t bound = LZ4F_compressFrameBound(input.size(), &prefs);
    std::string output(bound, '\0');

    size_t result = LZ4F_compressFrame(
        output.data(), output.size(), input.data(), input.size(), &prefs);

    if (LZ4F_isError(result))
        throw Frost_Recoverable_Error{fmt::format(
            "lz4.compress: compression failed ({})",
            LZ4F_getErrorName(result))};

    output.resize(result);
    return Value::create(std::move(output));
}

BUILTIN(decompress)
{
    REQUIRE_ARGS("lz4.decompress", TYPES(String));

    const auto& input = GET(0, String);

    LZ4F_dctx* dctx = nullptr;
    auto err = LZ4F_createDecompressionContext(&dctx, LZ4F_VERSION);
    if (LZ4F_isError(err))
        throw Frost_Recoverable_Error{
            "lz4.decompress: failed to create decompression context"};

    auto* src = reinterpret_cast<const void*>(input.data());
    size_t src_remaining = input.size();

    std::string output;
    std::array<char, 16384> buf;

    while (src_remaining > 0)
    {
        size_t dst_size = buf.size();
        size_t consumed = src_remaining;

        size_t ret = LZ4F_decompress(
            dctx, buf.data(), &dst_size, src, &consumed, nullptr);

        if (LZ4F_isError(ret))
        {
            LZ4F_freeDecompressionContext(dctx);
            throw Frost_Recoverable_Error{fmt::format(
                "lz4.decompress: decompression failed ({})",
                LZ4F_getErrorName(ret))};
        }

        output.append(buf.data(), dst_size);
        src = static_cast<const char*>(src) + consumed;
        src_remaining -= consumed;
    }

    LZ4F_freeDecompressionContext(dctx);
    return Value::create(std::move(output));
}

} // namespace frst::compression::lz4
