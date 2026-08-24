#include <frost/builtins-common.hpp>

#include <brotli/decode.h>
#include <brotli/encode.h>

#include <array>

namespace frst::compression::brotli
{

BUILTIN(compress)
{
    REQUIRE_ARGS("brotli.compress", TYPES(String),
                 OPTIONAL(PARAM("quality", TYPES(Int))));

    const auto& input = GET(0, String);

    int quality = BROTLI_DEFAULT_QUALITY;
    if (HAS(1))
    {
        quality = static_cast<int>(GET(1, Int));
        if (quality < BROTLI_MIN_QUALITY || quality > BROTLI_MAX_QUALITY)
            throw Frost_Recoverable_Error{fmt::format(
                "brotli.compress: quality must be between {} and {}",
                BROTLI_MIN_QUALITY, BROTLI_MAX_QUALITY)};
    }

    size_t output_size = BrotliEncoderMaxCompressedSize(input.size());
    if (output_size == 0)
        throw Frost_Recoverable_Error{"brotli.compress: input too large"};

    std::string output(output_size, '\0');

    auto ok = BrotliEncoderCompress(
        quality, BROTLI_DEFAULT_WINDOW, BROTLI_MODE_GENERIC, input.size(),
        reinterpret_cast<const uint8_t*>(input.data()), &output_size,
        reinterpret_cast<uint8_t*>(output.data()));

    if (not ok)
        throw Frost_Recoverable_Error{"brotli.compress: compression failed"};

    // brotli mutated output_size to be the actual compressed size
    output.resize(output_size);
    return Value::create(std::move(output));
}

BUILTIN(decompress)
{
    REQUIRE_ARGS("brotli.decompress", TYPES(String));

    const auto& input = GET(0, String);

    auto* state = BrotliDecoderCreateInstance(nullptr, nullptr, nullptr);
    if (not state)
        throw Frost_Recoverable_Error{
            "brotli.decompress: failed to create decoder"};

    auto available_in = input.size();
    auto* next_in = reinterpret_cast<const uint8_t*>(input.data());

    std::string output;
    std::array<uint8_t, 16384> buf;

    BrotliDecoderResult result;
    do
    {
        auto available_out = buf.size();
        auto* next_out = buf.data();

        result = BrotliDecoderDecompressStream(
            state, &available_in, &next_in, &available_out, &next_out, nullptr);

        if (result == BROTLI_DECODER_RESULT_ERROR)
        {
            auto* msg =
                BrotliDecoderErrorString(BrotliDecoderGetErrorCode(state));
            BrotliDecoderDestroyInstance(state);
            throw Frost_Recoverable_Error{fmt::format(
                "brotli.decompress: decompression failed ({})", msg)};
        }

        if (result == BROTLI_DECODER_RESULT_NEEDS_MORE_INPUT)
        {
            BrotliDecoderDestroyInstance(state);
            throw Frost_Recoverable_Error{"brotli.decompress: truncated input"};
        }

        output.append(reinterpret_cast<char*>(buf.data()),
                      buf.size() - available_out);
    } while (result != BROTLI_DECODER_RESULT_SUCCESS);

    BrotliDecoderDestroyInstance(state);
    return Value::create(std::move(output));
}

} // namespace frst::compression::brotli
