#include "builtins-common.h"

namespace frst
{

// arity is pre-checked by Builtin::call

template <Frost_Type T>
Value_Ptr is_impl(builtin_args_t args)
{
    return Value::create(args.at(0)->is<T>());
}

auto is_null = is_impl<Null>;
auto is_int = is_impl<Int>;
auto is_float = is_impl<Float>;
auto is_bool = is_impl<Bool>;
auto is_string = is_impl<String>;
auto is_array = is_impl<Array>;
auto is_map = is_impl<Map>;
auto is_function = is_impl<Function>;

Value_Ptr is_numeric(builtin_args_t args)
{
    return Value::create(args.at(0)->is_numeric());
}

Value_Ptr is_primitive(builtin_args_t args)
{
    return Value::create(args.at(0)->is_primitive());
}

Value_Ptr is_structured(builtin_args_t args)
{
    return Value::create(args.at(0)->is_structured());
}

void inject_type_checks(Symbol_Table& table)
{

    INJECT(is_null, 1, 1);
    INJECT(is_int, 1, 1);
    INJECT(is_float, 1, 1);
    INJECT(is_bool, 1, 1);
    INJECT(is_string, 1, 1);
    INJECT(is_array, 1, 1);
    INJECT(is_map, 1, 1);
    INJECT(is_function, 1, 1);
    INJECT(is_numeric, 1, 1);
    INJECT(is_primitive, 1, 1);
    INJECT(is_structured, 1, 1);
}

} // namespace frst
