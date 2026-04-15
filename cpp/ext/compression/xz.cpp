#include <frost/builtins-common.hpp>

#include <lzma.h>

#include <array>

namespace frst::compression::xz
{

BUILTIN(compress)
{
    REQUIRE_ARGS("xz.compress", TYPES(String),
                 OPTIONAL(PARAM("level", TYPES(Int))));

    const auto& input = GET(0, String);

    uint32_t preset = LZMA_PRESET_DEFAULT;
    if (HAS(1))
    {
        auto level = GET(1, Int);
        if (level < 0 || level > 9)
            throw Frost_Recoverable_Error{
                "xz.compress: level must be between 0 and 9"};
        preset = static_cast<uint32_t>(level);
    }

    size_t bound = lzma_stream_buffer_bound(input.size());
    std::string output(bound, '\0');
    size_t out_pos = 0;

    lzma_ret ret = lzma_easy_buffer_encode(
        preset, LZMA_CHECK_CRC64, nullptr,
        reinterpret_cast<const uint8_t*>(input.data()), input.size(),
        reinterpret_cast<uint8_t*>(output.data()), &out_pos, output.size());

    if (ret != LZMA_OK)
        throw Frost_Recoverable_Error{
            fmt::format("xz.compress: compression failed (error {})",
                        static_cast<int>(ret))};

    output.resize(out_pos);
    return Value::create(std::move(output));
}

BUILTIN(decompress)
{
    REQUIRE_ARGS("xz.decompress", TYPES(String));

    const auto& input = GET(0, String);

    lzma_stream stream = LZMA_STREAM_INIT;
    lzma_ret ret = lzma_auto_decoder(&stream, UINT64_MAX, LZMA_CONCATENATED);
    if (ret != LZMA_OK)
        throw Frost_Recoverable_Error{
            "xz.decompress: failed to initialize decoder"};

    stream.next_in = reinterpret_cast<const uint8_t*>(input.data());
    stream.avail_in = input.size();

    std::string output;
    std::array<uint8_t, 16384> buf;

    do
    {
        stream.next_out = buf.data();
        stream.avail_out = buf.size();

        ret = lzma_code(&stream, LZMA_FINISH);

        if (ret != LZMA_OK && ret != LZMA_STREAM_END)
        {
            lzma_end(&stream);
            throw Frost_Recoverable_Error{fmt::format(
                "xz.decompress: decompression failed (error {})",
                static_cast<int>(ret))};
        }

        output.append(reinterpret_cast<char*>(buf.data()),
                       buf.size() - stream.avail_out);
    }
    while (ret != LZMA_STREAM_END);

    lzma_end(&stream);
    return Value::create(std::move(output));
}

} // namespace frst::compression::xz
