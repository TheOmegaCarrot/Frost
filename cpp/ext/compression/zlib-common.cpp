#include "zlib-common.hpp"

#include <frost/builtins-common.hpp>

#include <zlib.h>

#include <array>

namespace frst::compression::zlib_common
{

Value_Ptr compress(std::string_view fn_name, builtin_args_t args,
                   int window_bits)
{
    REQUIRE_ARGS(fn_name, TYPES(String), OPTIONAL(PARAM("level", TYPES(Int))));

    const auto& input = GET(0, String);

    int level = Z_DEFAULT_COMPRESSION;
    if (HAS(1))
    {
        level = static_cast<int>(GET(1, Int));
        if (level < -1 || level > 9)
            throw Frost_Recoverable_Error{
                fmt::format("{}: level must be between -1 and 9", fn_name)};
    }

    z_stream stream{};
    if (deflateInit2(&stream, level, Z_DEFLATED, window_bits, 8,
                     Z_DEFAULT_STRATEGY)
        != Z_OK)
    {
        throw Frost_Recoverable_Error{
            fmt::format("{}: failed to initialize deflate stream", fn_name)};
    }

    stream.next_in = reinterpret_cast<Bytef*>(const_cast<char*>(input.data()));
    stream.avail_in = static_cast<uInt>(input.size());

    std::string output;
    output.resize(deflateBound(&stream, stream.avail_in));

    stream.next_out = reinterpret_cast<Bytef*>(output.data());
    stream.avail_out = static_cast<uInt>(output.size());

    int ret = deflate(&stream, Z_FINISH);
    deflateEnd(&stream);

    if (ret != Z_STREAM_END)
        throw Frost_Recoverable_Error{
            fmt::format("{}: compression failed", fn_name)};

    output.resize(stream.total_out);
    return Value::create(std::move(output));
}

Value_Ptr decompress(std::string_view fn_name, builtin_args_t args,
                     int window_bits)
{
    REQUIRE_ARGS(fn_name, TYPES(String));

    const auto& input = GET(0, String);

    z_stream stream{};
    if (inflateInit2(&stream, window_bits) != Z_OK)
        throw Frost_Recoverable_Error{
            fmt::format("{}: failed to initialize inflate stream", fn_name)};

    stream.next_in = reinterpret_cast<Bytef*>(const_cast<char*>(input.data()));
    stream.avail_in = static_cast<uInt>(input.size());

    std::string output;
    std::array<Bytef, 16384> buf;

    int ret;
    do
    {
        stream.next_out = buf.data();
        stream.avail_out = static_cast<uInt>(buf.size());

        ret = inflate(&stream, Z_NO_FLUSH);

        if (ret != Z_OK && ret != Z_STREAM_END)
        {
            inflateEnd(&stream);
            throw Frost_Recoverable_Error{
                fmt::format("{}: decompression failed ({})", fn_name,
                            stream.msg ? stream.msg : "unknown error")};
        }

        output.append(reinterpret_cast<char*>(buf.data()),
                      buf.size() - stream.avail_out);
    } while (ret != Z_STREAM_END);

    inflateEnd(&stream);
    return Value::create(std::move(output));
}

} // namespace frst::compression::zlib_common
