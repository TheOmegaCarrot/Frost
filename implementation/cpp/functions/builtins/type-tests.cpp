#include "builtins-common.hpp"
#include "frost/builtin.hpp"

namespace frst
{

// arity is pre-checked by Builtin::call

template <Frost_Type T>
BUILTIN(is_impl)
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

BUILTIN(is_nonnull)
{
    return is_null(args)->logical_not();
}

BUILTIN(is_numeric)
{
    return Value::create(args.at(0)->is_numeric());
}

BUILTIN(is_primitive)
{
    return Value::create(args.at(0)->is_primitive());
}

BUILTIN(is_structured)
{
    return Value::create(args.at(0)->is_structured());
}

BUILTIN(type)
{
    return Value::create(String{args.at(0)->type_name()});
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
    INJECT(is_nonnull, 1, 1);
    INJECT(is_numeric, 1, 1);
    INJECT(is_primitive, 1, 1);
    INJECT(is_structured, 1, 1);
    INJECT(type, 1, 1);
}

} // namespace frst
