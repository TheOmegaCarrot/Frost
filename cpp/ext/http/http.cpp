#include <frost/extensions-common.hpp>

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <boost/url.hpp>

#include "request.hpp"

namespace frst
{

namespace http
{

BUILTIN(request)
{
    REQUIRE_ARGS("http.request", TYPES(Map));

    return do_http_request(GET(0, Map));
}

BUILTIN(parse_url)
{
    REQUIRE_ARGS("http.parse_url", TYPES(String));

    const auto& url_string = GET(0, String);

    auto result = boost::urls::parse_uri(url_string);
    if (result.has_error())
    {
        throw Frost_Recoverable_Error{
            fmt::format("http.parse_url: invalid URL: {}", url_string)};
    }

    const auto& url = result.value();

    STRINGS(host, path, tls, port, query);

    bool is_tls = true;
    if (url.has_scheme())
        is_tls = url.scheme_id() != boost::urls::scheme::http;

    Map uri_map{
        {strings.host, Value::create(std::string{url.host()})},
        {strings.path,
         Value::create(url.path().empty() ? std::string{"/"}
                                          : std::string{url.path()})},
        {strings.tls, Value::create(is_tls)},
    };

    if (url.has_port())
        uri_map.emplace(strings.port,
                        Value::create(Int{std::stoi(std::string{url.port()})}));

    if (url.has_query())
    {
        Map query_map;
        for (const auto& param : url.params())
        {
            auto key = Value::create(std::string{param.key});
            if (param.has_value)
            {
                query_map.emplace(std::move(key),
                                  Value::create(std::string{param.value}));
            }
            else
            {
                query_map.emplace(std::move(key), Value::null());
            }
        }
        uri_map.emplace(strings.query,
                        Value::create(Value::trusted, std::move(query_map)));
    }

    return Value::create(Value::trusted, std::move(uri_map));
}

} // namespace http

REGISTER_EXTENSION(http, ENTRY(request), ENTRY(parse_url));

} // namespace frst
