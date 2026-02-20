#ifndef FROST_BUILTINS_ASYNC_HTTP_REQUEST_HPP
#define FROST_BUILTINS_ASYNC_HTTP_REQUEST_HPP

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <chrono>
#include <expected>
#include <future>
#include <optional>
#include <thread>

#include <boost/asio/io_context.hpp>
#include <boost/beast/http/verb.hpp>

#include "../async.hpp"

namespace frst::http
{

struct Header
{
    std::string key;
    std::string value;
};

struct Outgoing_Request
{
    struct URI
    {
        std::string host;
        std::string path = "/";
        bool tls = true;
        std::uint16_t port = 0;

        struct Query_Parameter
        {
            std::string key;
            std::optional<std::string> value;
        };

        std::vector<Query_Parameter> query_parameters;

    } uri;

    std::vector<Header> headers;
    boost::beast::http::verb method = boost::beast::http::verb::get;
    std::optional<std::string> body;
    std::chrono::milliseconds timeout = std::chrono::seconds{10};
    bool verify_tls = true;
    std::optional<std::string> ca_file;
    std::optional<std::string> ca_path;
    bool use_system_ca = true;
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
        std::uint32_t code;
        std::vector<Header> headers;
        std::string body;
    };

    // result.has_value() == ok at top-level of Frost result map
    std::expected<Reply, Error> result;
};

Value_Ptr do_http_request(const Map& request_spec);
} // namespace frst::http

#endif
