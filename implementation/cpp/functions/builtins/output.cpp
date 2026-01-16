#include "builtins-common.hpp"

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

namespace frst
{

namespace
{

std::string mformat_impl(const String& fmt_str, const Map& repl_map)
{
}

} // namespace

Value_Ptr mformat(builtin_args_t args)
{
    REQUIRE_ARGS(mformat, PARAM("format string", TYPES(String)),
                 PARAM("replacement map", TYPES(Map)));

    return Value::create(mformat_impl(args.at(0)->raw_get<String>(),
                                      args.at(1)->raw_get<Map>()));
}

void inject_output(Symbol_Table& table)
{
    INJECT(mformat, 2, 2);
}
} // namespace frst
