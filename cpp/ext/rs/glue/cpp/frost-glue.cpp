#include "frost-glue.hpp"

#include <frost/value.hpp>

namespace frst::rs
{

// frst::rs::Value is const frst::Value (via the typedef in the header),
// so Value_Ptr and shared_ptr<Value> are the same type.

// ---- Factories ----

std::shared_ptr<Value> value_null()
{
    return frst::Value::null();
}

std::shared_ptr<Value> value_from_int(int64_t val)
{
    return frst::Value::create(Int{val});
}

std::shared_ptr<Value> value_from_float(double val)
{
    return frst::Value::create(Float{val});
}

std::shared_ptr<Value> value_from_bool(bool val)
{
    return frst::Value::create(Bool{val});
}

std::shared_ptr<Value> value_from_string(const std::string& val)
{
    return frst::Value::create(String{val});
}

// ---- Type checks ----

bool value_is_null(const Value& val)
{
    return val.is<Null>();
}

bool value_is_int(const Value& val)
{
    return val.is<Int>();
}

bool value_is_float(const Value& val)
{
    return val.is<Float>();
}

bool value_is_bool(const Value& val)
{
    return val.is<Bool>();
}

bool value_is_string(const Value& val)
{
    return val.is<String>();
}

bool value_is_array(const Value& val)
{
    return val.is<Array>();
}

bool value_is_map(const Value& val)
{
    return val.is<Map>();
}

bool value_is_function(const Value& val)
{
    return val.is<Function>();
}

// ---- Accessors ----

int64_t value_get_int(const Value& val)
{
    return val.raw_get<Int>();
}

double value_get_float(const Value& val)
{
    return val.raw_get<Float>();
}

bool value_get_bool(const Value& val)
{
    return val.raw_get<Bool>();
}

const std::string& value_get_string(const Value& val)
{
    return val.raw_get<String>();
}

// ---- Stringification ----

std::unique_ptr<std::string> value_to_string(const Value& val)
{
    return std::make_unique<std::string>(val.to_internal_string());
}

std::unique_ptr<std::string> value_type_name(const Value& val)
{
    return std::make_unique<std::string>(val.type_name());
}

} // namespace frst::rs
