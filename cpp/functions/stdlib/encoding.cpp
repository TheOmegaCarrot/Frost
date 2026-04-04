#include <frost/builtins-common.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <cppcodec/base64_rfc4648.hpp>
#include <cppcodec/base64_url.hpp>

#include <openssl/crypto.h>

#include <boost/scope_exit.hpp>

#include <fmt/ranges.h>

namespace frst
{

namespace encoding
{

namespace b64
{

BUILTIN(encode)
{
    REQUIRE_ARGS("encoding.b64.encode", TYPES(String));

    try
    {
        return Value::create(cppcodec::base64_rfc4648::encode(GET(0, String)));
    }
    catch (const std::exception& e)
    {
        throw Frost_Recoverable_Error{String{e.what()}};
    }
}

BUILTIN(decode)
{
    REQUIRE_ARGS("encoding.b64.decode", TYPES(String));

    try
    {
        return Value::create(String(
            std::from_range, cppcodec::base64_rfc4648::decode(GET(0, String))));
    }
    catch (const std::exception& e)
    {
        throw Frost_Recoverable_Error{String{e.what()}};
    }
}

BUILTIN(urlencode)
{
    REQUIRE_ARGS("encoding.b64.urlencode", TYPES(String));

    try
    {
        return Value::create(cppcodec::base64_url::encode(GET(0, String)));
    }
    catch (const std::exception& e)
    {
        throw Frost_Recoverable_Error{String{e.what()}};
    }
}

BUILTIN(urldecode)
{
    REQUIRE_ARGS("encoding.b64.urldecode", TYPES(String));

    try
    {
        return Value::create(String(
            std::from_range, cppcodec::base64_url::decode(GET(0, String))));
    }
    catch (const std::exception& e)
    {
        throw Frost_Recoverable_Error{String{e.what()}};
    }
}

} // namespace b64

namespace hex
{

BUILTIN(encode)
{
    REQUIRE_ARGS("encoding.hex.encode", TYPES(String));

    const auto& input = GET(0, String);
    return Value::create(fmt::format(
        "{:02x}", fmt::join(std::span{reinterpret_cast<const unsigned char*>(
                                          input.data()),
                                      input.size()},
                            "")));
}

BUILTIN(decode)
{
    REQUIRE_ARGS("encoding.hex.decode", TYPES(String));

    const auto& input = GET(0, String);

    long len = 0;
    auto* buf = OPENSSL_hexstr2buf(input.c_str(), &len);
    BOOST_SCOPE_EXIT_ALL(&)
    {
        OPENSSL_free(buf);
    };

    if (not buf)
        throw Frost_Recoverable_Error{
            "encoding.hex.decode: invalid hex string"};

    String result(reinterpret_cast<char*>(buf), len);
    return Value::create(std::move(result));
}

} // namespace hex

BUILTIN(fmt_int)
{
    REQUIRE_ARGS("encoding.fmt_int", PARAM("number", TYPES(Int)),
                 PARAM("base", TYPES(Int)));

    const Int input = GET(0, Int);
    const Int base = GET(1, Int);

    if (base < 2 || base > 36)
    {
        throw Frost_Recoverable_Error{fmt::format(
            "fmt_int given base of {}, but base must be in range [2, 36]",
            base)};
    }

    char buf[66]{};

    const auto [ptr, ec] =
        std::to_chars(std::begin(buf), std::end(buf), input, base);

    // This really shouldn't happen, the buffer is big enough for any input
    if (ec != std::errc{})
        THROW_UNREACHABLE;

    return Value::create(String{std::begin(buf), ptr});
}

BUILTIN(parse_int)
{
    REQUIRE_ARGS("encoding.parse_int", PARAM("number", TYPES(String)),
                 PARAM("base", TYPES(Int)));

    const String& input = GET(0, String);
    const Int base = GET(1, Int);

    if (base < 2 || base > 36)
    {
        throw Frost_Recoverable_Error{fmt::format(
            "parse_int given base of {}, but base must be in range [2, 36]",
            base)};
    }

    Int result{};
    const auto [ptr, ec] = std::from_chars(
        input.data(), input.data() + input.size(), result, base);

    if ((ec == std::errc::invalid_argument)
        || (ptr != input.data() + input.size()))
    {
        throw Frost_Recoverable_Error{fmt::format(
            "parse_int expected numeric string in base {}, but got \"{}\"",
            base, input)};
    }

    if (ec == std::errc::result_out_of_range)
    {
        throw Frost_Recoverable_Error{fmt::format(
            "parse_int cannot parse \"{}\", which is out of range", input)};
    }

    return Value::create(result);
}

BUILTIN(to_bytes)
{
    REQUIRE_ARGS("encoding.to_bytes", TYPES(String));

    const String& input = GET(0, String);

    Array exploded =
        input
        | std::views::transform([](char c) {
              return Value::create(Int{static_cast<unsigned char>(c)});
          })
        | std::ranges::to<Array>();

    return Value::create(std::move(exploded));
}

BUILTIN(from_bytes)
{
    REQUIRE_ARGS("encoding.from_bytes", TYPES(Array));

    const Array& input = GET(0, Array);

    String acc;
    acc.reserve(input.size());
    for (const auto& elem : input)
    {
        acc.push_back(elem->get<Int>()
                          .or_else([&] -> std::optional<Int> {
                              throw Frost_Recoverable_Error{fmt::format(
                                  "Function encoding.from_bytes expected Array "
                                  "of Int, but found: {}",
                                  elem->type_name())};
                          })
                          .transform([&](Int i) {
                              if (i <= 255 && i >= 0)
                                  return static_cast<char>(i);
                              throw Frost_Recoverable_Error{fmt::format(
                                  "Function encoding.from_bytes expected Array "
                                  "of Int in range [0, 255], but got: {}",
                                  i)};
                          })
                          .value());
    }

    return Value::create(std::move(acc));
}

} // namespace encoding

STDLIB_MODULE(encoding,
              {"b64"_s, Value::create(Value::trusted,
                                      Map{
                                          NS_ENTRY(b64, encode),
                                          NS_ENTRY(b64, decode),
                                          NS_ENTRY(b64, urlencode),
                                          NS_ENTRY(b64, urldecode),
                                      })},
              {"hex"_s, Value::create(Value::trusted,
                                      Map{
                                          NS_ENTRY(hex, encode),
                                          NS_ENTRY(hex, decode),
                                      })},
              ENTRY(fmt_int), ENTRY(parse_int), ENTRY(to_bytes),
              ENTRY(from_bytes))

} // namespace frst
