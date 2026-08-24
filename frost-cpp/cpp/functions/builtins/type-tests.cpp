#include "frost/builtin.hpp"
#include <frost/builtins-common.hpp>

namespace frst
{

#define TYPE_TEST(FNAME, TYPE)                                                 \
    BUILTIN(FNAME)                                                             \
    {                                                                          \
        REQUIRE_ARGS(#FNAME, ANY);                                             \
        return Value::create(args.at(0)->is<TYPE>());                          \
    }

TYPE_TEST(is_null, Null)
TYPE_TEST(is_int, Int)
TYPE_TEST(is_float, Float)
TYPE_TEST(is_bool, Bool)
TYPE_TEST(is_string, String)
TYPE_TEST(is_array, Array)
TYPE_TEST(is_map, Map)
TYPE_TEST(is_function, Function)

#undef TYPE_TEST

BUILTIN(is_nonnull)
{
    REQUIRE_ARGS("is_nonnull", ANY);
    return Value::create(not args.at(0)->is<Null>());
}

BUILTIN(is_numeric)
{
    REQUIRE_ARGS("is_numeric", ANY);
    return Value::create(args.at(0)->is_numeric());
}

BUILTIN(is_primitive)
{
    REQUIRE_ARGS("is_primitive", ANY);
    return Value::create(args.at(0)->is_primitive());
}

BUILTIN(is_structured)
{
    REQUIRE_ARGS("is_structured", ANY);
    return Value::create(args.at(0)->is_structured());
}

BUILTIN(type)
{
    REQUIRE_ARGS("type", ANY);
    return Value::create(String{args.at(0)->type_name()});
}

void inject_type_checks(Symbol_Table& table)
{

    INJECT(is_null);
    INJECT(is_int);
    INJECT(is_float);
    INJECT(is_bool);
    INJECT(is_string);
    INJECT(is_array);
    INJECT(is_map);
    INJECT(is_function);
    INJECT(is_nonnull);
    INJECT(is_numeric);
    INJECT(is_primitive);
    INJECT(is_structured);
    INJECT(type);
}

} // namespace frst
