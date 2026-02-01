#include <frost/builtins-common.hpp>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

namespace frst
{

BUILTIN(debug_dump)
{
    if (const auto& arg = args.at(0); arg->is<Function>())
    {
        return Value::create(arg->raw_get<Function>()->debug_dump());
    }
    else
    {
        return arg->to_string();
    }
}

#ifdef assert
#undef assert
#endif
BUILTIN(assert)
{
    REQUIRE_ARGS("assert", PARAM("condition", ANY),
                 OPTIONAL(PARAM("message", TYPES(String))));

    if (not args.at(0)->truthy())
    {
        if (args.size() == 1)
            throw Frost_Recoverable_Error{"Failed assertion"};
        else
            throw Frost_Recoverable_Error{
                fmt::format("Failed assertion: {}", GET(1, String))};
    }
    return args.at(0);
}

void inject_debug_helpers(Symbol_Table& table)
{
    INJECT(debug_dump, 1, 1);
    INJECT(assert, 1, 2);
}

} // namespace frst
