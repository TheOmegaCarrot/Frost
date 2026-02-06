#ifndef FROST_BUILTINS_HTTP_IMPL_REQUEST_HPP
#define FROST_BUILTINS_HTTP_IMPL_REQUEST_HPP

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <chrono>
#include <expected>
#include <optional>

namespace frst::http
{

struct Header
{
    std::string key;
    std::variant<std::string, std::vector<std::string>> value;
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
            std::optional<std::variant<std::string, std::vector<std::string>>>
                value;
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

struct Incoming_Response
{
    struct Error
    {
        std::string category; // maybe leave out? overlaps with phase
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

Value_Ptr do_http_request(const Map& request_spec);
} // namespace frst::http

#endif
