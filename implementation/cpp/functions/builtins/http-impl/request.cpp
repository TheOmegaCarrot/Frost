#include "request.hpp"
#include "frost/value.hpp"
#include <limits>

namespace frst::http
{

namespace
{

// lil helper for optional::or_else :)
template <typename T = std::string>
auto thrower(const std::string& err)
{
    return [&] -> std::optional<T> {
        throw Frost_Recoverable_Error{err};
    };
}

std::vector<Outgoing_Request::Endpoint::Query_Parameter> parse_query_parameters(
    const Value_Ptr& query_parameters_spec_val)
{
}

Outgoing_Request::Endpoint parse_endpoint(const Value_Ptr& endpoint_spec_val)
{
    Outgoing_Request::Endpoint endpoint;

    if (not endpoint_spec_val->is<Map>())
        throw Frost_Recoverable_Error{"http.request: endpoint must be a Map"};

    const auto& endpoint_spec = endpoint_spec_val->raw_get<Map>();

    // required fields checklist
    bool uri_read = false;

    for (const auto& [k_val, v_val] : endpoint_spec)
    {
        if (not k_val->is<String>())
        {
            throw Frost_Recoverable_Error{
                fmt::format("http.request: unexpected key in endpoint: {}",
                            k_val->to_internal_string({.in_structure = true}))};
        }

        const auto& key = k_val->raw_get<String>();

        if (key == "uri")
        {
            uri_read = true;
            endpoint.uri =
                v_val->get<String>()
                    .or_else(
                        thrower("http.request: endpoint.uri must be a String"))
                    .value();
        }
        else if (key == "path")
        {
            endpoint.path =
                v_val->get<String>()
                    .or_else(
                        thrower("http.request: endpoint.path must be a String"))
                    .value();
        }
        else if (key == "tls")
        {
            endpoint.tls = v_val->get<Bool>()
                               .or_else(thrower<Bool>(
                                   "http.request: endpoint.tls must be a Bool"))
                               .value();
        }
        else if (key == "port")
        {
            endpoint.port =
                v_val->get<Int>()
                    .or_else(thrower<Int>(
                        "http.request: endpoint.port must be an Int"))
                    .transform([](Int port) {
                        if (port > std::numeric_limits<std::uint16_t>::max())
                        {
                            throw Frost_Recoverable_Error{
                                fmt::format("http.request: endpoint.port value "
                                            "{} is out of range",
                                            port)};
                        }

                        return port;
                    })
                    .value();
        }
        else if (key == "query")
        {
            endpoint.query_parameters = parse_query_parameters(v_val);
        }
        else
        {
            throw Frost_Recoverable_Error{fmt::format(
                "http.request: got unexpected key in endpoint: {}", key)};
        }
    }

    if (not uri_read)
    {
        throw Frost_Recoverable_Error{
            "http.request: missing required field: endpoint.uri"};
    }

    if (endpoint.port == 0)
        endpoint.port = endpoint.tls ? 443 : 80;

    return endpoint;
}

std::vector<Header> parse_headers(const Value_Ptr& headers_spec)
{
}

std::string parse_method(const Value_Ptr& method_spec)
{
    // normalize to all-caps
}

Outgoing_Request parse_request(const Map& request_spec)
{

    Outgoing_Request request;

    // required fields checklist
    bool endpoint_read = false;

    for (const auto& [k_val, v_val] : request_spec)
    {
        if (not k_val->is<String>())
        {
            throw Frost_Recoverable_Error{
                fmt::format("http.request: unexpected key: {}",
                            k_val->to_internal_string({.in_structure = true}))};
        }

        const auto& key = k_val->raw_get<String>();

        if (key == "endpoint")
        {
            endpoint_read = true;
            request.endpoint = parse_endpoint(v_val);
        }
        else if (key == "headers")
        {
            request.headers = parse_headers(v_val);
        }
        else if (key == "method")
        {
            request.method = parse_method(v_val);
        }
        else if (key == "body")
        {
            request.body =
                v_val->get<String>()
                    .or_else(thrower("http.request: body must be a String"))
                    .value();
        }
        else if (key == "timeout_ms")
        {
            request.timeout = std::chrono::milliseconds{
                v_val->get<Int>()
                    .or_else(
                        thrower<Int>("http.request: timeout must be an Int"))
                    .value()};
        }
        else if (key == "verify_tls")
        {
            request.verify_tls =
                v_val->get<Bool>()
                    .or_else(thrower<Bool>(
                        "http.request: verify_tls must be a Bool"))
                    .value();
        }
        else if (key == "ca_file")
        {
            request.ca_file =
                v_val->get<String>()
                    .or_else(thrower("http.request: ca_file must be a String"))
                    .value();
        }
        else if (key == "ca_path")
        {
            request.ca_path =
                v_val->get<String>()
                    .or_else(thrower("http.request: ca_path must be a String"))
                    .value();
        }
        else
        {
            throw Frost_Recoverable_Error{
                fmt::format("http.request: got unexpected key: {}", key)};
        }
    }

    if (not endpoint_read)
        throw Frost_Recoverable_Error{
            "http.request: missing required field: endpoint"};

    return request;
}

} // namespace

Value_Ptr do_http_request(const Map& request_spec)
{
    auto request = parse_request(request_spec);
}

} // namespace frst::http
