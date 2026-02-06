#ifndef FROST_BUILTINS_HTTP_IMPL_REQUEST_HPP
#define FROST_BUILTINS_HTTP_IMPL_REQUEST_HPP

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <chrono>
#include <expected>
#include <optional>

#include <boost/asio.hpp>
#include <boost/cobalt.hpp>

namespace frst::http
{

struct Header
{
    std::string key;
    std::string value;
};

struct Outgoing_Request
{
    struct Endpoint
    {
        std::string uri;
        std::string path = "/";
        bool tls = true;
        std::uint16_t port = 0;

        struct Query_Parameter
        {
            std::string key;
            std::optional<std::string> value;
        };

        std::vector<Query_Parameter> query_parameters;

    } endpoint;

    std::vector<Header> headers;
    std::string method = "GET";
    std::optional<std::string> body;
    std::chrono::milliseconds timeout = std::chrono::seconds{10};
    bool verify_tls = true;
    std::optional<std::string> ca_file;
    std::optional<std::string> ca_path;
};

struct Request_Result
{
    struct Error
    {
        std::string category;
        std::string message;
        std::string phase;
    };

    struct Reply
    {
        std::uint16_t code;
        std::vector<Header> headers;
        std::string body;
    };

    // result.has_value() == ok at top-level of Frost result map
    std::expected<Reply, Error> result;
};

struct Request_Task
{
    boost::asio::io_context ioc;
    std::future<Request_Result> future;
    std::jthread worker;

    std::once_flag cache_once;
    Value_Ptr cache;

    bool is_ready() const
    {
        return future.wait_for(std::chrono::seconds{0})
               == std::future_status::ready;
    }

    Value_Ptr get();
};

Value_Ptr do_http_request(const Map& request_spec);
} // namespace frst::http

#endif
