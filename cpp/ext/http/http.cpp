#include <frost/extensions-common.hpp>

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <charconv>

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
    {
        auto port_str = url.port();
        Int port_val = 0;
        auto [ptr, ec] = std::from_chars(
            port_str.data(), port_str.data() + port_str.size(), port_val);
        if (ec != std::errc{})
            throw Frost_Recoverable_Error{
                fmt::format("http.parse_url: invalid port: {}", port_str)};
        uri_map.emplace(strings.port, Value::create(port_val));
    }

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

BUILTIN(build_url)
{
    REQUIRE_ARGS("http.build_url", TYPES(Map));

    auto uri = parse_uri(args[0]);

    boost::urls::url url;
    url.set_scheme(uri.tls ? "https" : "http");
    url.set_host(uri.host);

    auto default_port = uri.tls ? 443 : 80;
    if (uri.port != default_port)
        url.set_port(std::to_string(uri.port));

    url.set_path(uri.path);

    if (not uri.query_parameters.empty())
    {
        auto params = url.params();
        for (const auto& qp : uri.query_parameters)
        {
            if (qp.value.has_value())
                params.append({qp.key, qp.value.value()});
            else
                params.append({qp.key, {}, false});
        }
    }

    return Value::create(std::string{url.buffer()});
}

} // namespace http

REGISTER_EXTENSION(http, ENTRY(request), ENTRY(parse_url), ENTRY(build_url));

} // namespace frst
