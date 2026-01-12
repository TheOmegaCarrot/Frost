#include "builtins-common.hpp"

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

namespace frst
{

Value_Ptr debug_dump(builtin_args_t args)
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
Value_Ptr assert(builtin_args_t args)
{
    REQUIRE_ARGS(assert, PARAM("condition", ANY),
                 OPTIONAL(PARAM("message", TYPES(String))));

    if (not args.at(0)->as<Bool>().value())
    {
        if (args.size() == 1)
            throw Frost_Error{"Failed assertion"};
        else
            throw Frost_Error{fmt::format("Failed assertion: {}",
                                          args.at(1)->raw_get<String>())};
    }
    return args.at(0);
}

void inject_debug_helpers(Symbol_Table& table)
{
    INJECT(debug_dump, 1, 1);
    INJECT(assert, 1, 2);
}

} // namespace frst
