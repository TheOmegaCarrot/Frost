#include "request.hpp"

#include <frost/builtins-common.hpp>
#include <frost/value.hpp>

#include <boost/algorithm/string.hpp>
#include <boost/hof/lift.hpp>

#include <algorithm>
#include <limits>

namespace frst::http
{

std::shared_ptr<Request_Task> async_do_http_request(
    const Outgoing_Request& request)
{
    // just spawn coro and build task
}

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

} // namespace

Value_Ptr request_result_to_value(Request_Result&& request_result)
{
    STRINGS(ok, error, category, message, phase, response, code, headers, body);

    Map top{{strings.ok, Value::create(request_result.result.has_value())}};

    if (request_result.result)
    {
        auto& resp = request_result.result.value();
        Map response_map{
            {strings.code, Value::create(resp.code)},
            {strings.body, Value::create(std::move(resp.body))},
        };

        for (auto& [key, _] : resp.headers)
            boost::algorithm::to_lower(key); // normalize

        std::ranges::stable_sort(resp.headers, {}, &Header::key);

        Map headers_out;

        for (auto group :
             resp.headers
                 | std::views::chunk_by([](const auto& a, const auto& b) {
                       return a.key == b.key;
                   }))
        {
            auto values = group
                          | std::views::transform(&Header::value)
                          | std::views::as_rvalue
                          | std::views::transform(BOOST_HOF_LIFT(Value::create))
                          | std::ranges::to<std::vector<Value_Ptr>>();

            Value_Ptr header_map_value;
            if (values.size() == 1)
                header_map_value = values.at(0);
            else
                header_map_value = Value::create(std::move(values));

            headers_out.emplace(Value::create(std::move(group.front().key)),
                                std::move(header_map_value));
        }

        response_map.emplace(strings.headers,
                             Value::create(std::move(headers_out)));

        top.emplace(strings.response, Value::create(std::move(response_map)));
    }
    else
    {
        auto& err = request_result.result.error();
        top.emplace(
            strings.error,
            Value::create(Map{
                {strings.category, Value::create(std::move(err.category))},
                {strings.phase, Value::create(std::move(err.phase))},
                {strings.message, Value::create(std::move(err.message))},
            }));
    }

    return Value::create(std::move(top));
}

Value_Ptr Request_Task::get()
{
    Request_Result result = future.get();
    std::call_once(cache_once, [&] {
        cache = request_result_to_value(std::move(result));
    });
    return cache;
}

namespace
{

std::vector<Outgoing_Request::Endpoint::Query_Parameter> parse_query_parameters(
    const Value_Ptr& query_spec_val)
{
    if (not query_spec_val->is<Map>())
        throw Frost_Recoverable_Error{
            "http.request: endpoint.query must be a Map"};

    const auto& query_spec = query_spec_val->raw_get<Map>();

    std::vector<Outgoing_Request::Endpoint::Query_Parameter> result;

    for (const auto& [k_val, v_val] : query_spec)
    {
        if (not k_val->is<String>())
        {
            throw Frost_Recoverable_Error{fmt::format(
                "http.request: unexpected key in endpoint.query: {}",
                k_val->to_internal_string({.in_structure = true}))};
        }

        const auto& key = k_val->raw_get<String>();
        if (v_val->is<String>())
        {
            result.emplace_back(key, v_val->raw_get<String>());
        }
        else if (v_val->is<Null>())
        {
            result.emplace_back(key, std::nullopt);
        }
        else if (v_val->is<Array>())
        {
            const auto& values = v_val->raw_get<Array>();

            for (const auto& value : values)
            {
                result.emplace_back(key, value->get<String>()
                                             .or_else(thrower(fmt::format(
                                                 "http.request: "
                                                 "endpoint.query array values "
                                                 "must be Strings, got {}",
                                                 value->type_name())))
                                             .value());
            }
        }
        else
        {
            throw Frost_Recoverable_Error{
                fmt::format("http.request: endpoint.query values must be "
                            "String, Array, or Null, got: {}",
                            v_val->type_name())};
        }
    }

    return result;
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
            // if uri is invalid, it will be handled later
            uri_read = true;
            endpoint.uri =
                v_val->get<String>()
                    .or_else(
                        thrower("http.request: endpoint.uri must be a String"))
                    .value();
        }
        else if (key == "path")
        {
            // if path is invalid, it will be handled later
            endpoint.path =
                v_val->get<String>()
                    .or_else(
                        thrower("http.request: endpoint.path must be a String"))
                    .value();

            if (not endpoint.path.starts_with("/"))
                endpoint.path = '/' + endpoint.path;
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
                        if ((port > std::numeric_limits<std::uint16_t>::max())
                            || (port <= 0))
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
    if (not headers_spec->is<Map>())
        throw Frost_Recoverable_Error{"http.request: headers must be a Map"};

    const auto& headers = headers_spec->raw_get<Map>();

    std::vector<Header> result;

    for (const auto& [k_val, v_val] : headers)
    {
        if (not k_val->is<String>())
        {
            throw Frost_Recoverable_Error{
                fmt::format("http.request: headers Map keys must be "
                            "Strings, got {}",
                            k_val->type_name())};
        }

        const auto& key = k_val->raw_get<String>();

        if (v_val->is<String>())
        {
            result.emplace_back(key, v_val->raw_get<String>());
        }
        else if (v_val->is<Array>())
        {
            const auto& vals = v_val->raw_get<Array>();
            for (const auto& val : vals)
            {
                result.emplace_back(
                    key, val->get<String>()
                             .or_else(thrower(fmt::format(
                                 "http.request: headers Array values "
                                 "must be Strings, got {}",
                                 val->type_name())))
                             .value());
            }
        }
        else
        {
            throw Frost_Recoverable_Error{
                fmt::format("http.request: headers got unexpected value: {}",
                            v_val->to_internal_string())};
        }
    }

    return result;
}

std::string parse_method(const Value_Ptr& method_spec)
{
    std::string method =
        method_spec->get<String>()
            .or_else(thrower(
                fmt::format("http.request: method must be a String, got: {}",
                            method_spec->to_internal_string())))
            .value();

    boost::algorithm::to_upper(method);

    return method;
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
                    .transform([](Int timeout) {
                        if (timeout <= 0)
                            throw Frost_Recoverable_Error{
                                "http.request: timeout must be positive"};
                        return timeout;
                    })
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

    std::shared_ptr<Request_Task> task = async_do_http_request(request);
}

} // namespace frst::http
