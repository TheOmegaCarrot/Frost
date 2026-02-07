#include <frost/builtins-common.hpp>

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

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
} // namespace http

void inject_http(Symbol_Table& table)
{
    using namespace http;
    INJECT_MAP(http, ENTRY(request, 1, 1));
}
} // namespace frst
