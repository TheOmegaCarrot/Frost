#include <frost/builtins-common.hpp>

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <cppcodec/base64_rfc4648.hpp>
#include <cppcodec/base64_url.hpp>

#include "http-impl/request.hpp"

namespace frst
{

namespace http
{

BUILTIN(request)
{
    REQUIRE_ARGS("http.request", TYPES(Map));

    return do_http_request(GET(0, Map));
}

BUILTIN(b64_encode)
{
    REQUIRE_ARGS("http.b64_encode", TYPES(String));

    return Value::create(cppcodec::base64_rfc4648::encode(GET(0, String)));
}

BUILTIN(b64_decode)
{
    REQUIRE_ARGS("http.b64_decode", TYPES(String));

    return Value::create(String(
        std::from_range, cppcodec::base64_rfc4648::decode(GET(0, String))));
}

BUILTIN(b64_urlencode)
{
    REQUIRE_ARGS("http.b64_encode", TYPES(String));

    return Value::create(cppcodec::base64_url::encode(GET(0, String)));
}

BUILTIN(b64_urldecode)
{
    REQUIRE_ARGS("http.b64_decode", TYPES(String));

    return Value::create(
        String(std::from_range, cppcodec::base64_url::decode(GET(0, String))));
}

} // namespace http

void inject_http(Symbol_Table& table)
{
    using namespace http;
    INJECT_MAP(http, ENTRY(request, 1, 1), ENTRY(b64_encode, 1, 1),
               ENTRY(b64_decode, 1, 1), ENTRY(b64_urlencode, 1, 1),
               ENTRY(b64_urldecode, 1, 1));
}
} // namespace frst
