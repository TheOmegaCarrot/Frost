#include <frost/extensions-common.hpp>

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

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

} // namespace http

DECLARE_EXTENSION(http)
{
    using namespace http;
    CREATE_EXTENSION(ENTRY(request, 1));
}
} // namespace frst
