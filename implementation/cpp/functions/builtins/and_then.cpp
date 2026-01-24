#include "builtins-common.hpp"

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

namespace frst
{

BUILTIN(and_then)
{
    REQUIRE_ARGS("and_then", ANY, TYPES(Function));

    if (args.at(0)->is<Null>())
        return Value::null();

    return GET(1, Function)->call({args.at(0)});
}

BUILTIN(or_else)
{
    REQUIRE_ARGS("or_else", ANY, TYPES(Function));

    if (args.at(0)->is<Null>())
        return GET(1, Function)->call({});

    return args.at(0);
}

void inject_and_then(Symbol_Table& table)
{
    INJECT(and_then, 2, 2);
    INJECT(or_else, 2, 2);
}
} // namespace frst
