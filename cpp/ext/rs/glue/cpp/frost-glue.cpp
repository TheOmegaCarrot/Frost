#include "frost-glue.hpp"

#include <frost/value.hpp>

namespace frst::rs
{

// ValuePtr is const frst::Value under a forward-declared name so the header
// stays free of frost includes.  These two helpers bridge the gap.

static const Value& as_value(const ValuePtr& v)
{
    return reinterpret_cast<const Value&>(v);
}

static std::shared_ptr<ValuePtr> wrap(Value_Ptr v)
{
    return reinterpret_cast<std::shared_ptr<ValuePtr>&>(v);
}

// ---- Factories ----

std::shared_ptr<ValuePtr> value_null()
{
    return wrap(Value::null());
}

std::shared_ptr<ValuePtr> value_from_int(int64_t val)
{
    return wrap(Value::create(Int{val}));
}

std::shared_ptr<ValuePtr> value_from_float(double val)
{
    return wrap(Value::create(Float{val}));
}

std::shared_ptr<ValuePtr> value_from_bool(bool val)
{
    return wrap(Value::create(Bool{val}));
}

std::shared_ptr<ValuePtr> value_from_string(const std::string& val)
{
    return wrap(Value::create(String{val}));
}

// ---- Type checks ----

bool value_is_null(const ValuePtr& val)
{
    return as_value(val).is<Null>();
}

bool value_is_int(const ValuePtr& val)
{
    return as_value(val).is<Int>();
}

bool value_is_float(const ValuePtr& val)
{
    return as_value(val).is<Float>();
}

bool value_is_bool(const ValuePtr& val)
{
    return as_value(val).is<Bool>();
}

bool value_is_string(const ValuePtr& val)
{
    return as_value(val).is<String>();
}

bool value_is_array(const ValuePtr& val)
{
    return as_value(val).is<Array>();
}

bool value_is_map(const ValuePtr& val)
{
    return as_value(val).is<Map>();
}

bool value_is_function(const ValuePtr& val)
{
    return as_value(val).is<Function>();
}

// ---- Accessors ----

int64_t value_get_int(const ValuePtr& val)
{
    return as_value(val).raw_get<Int>();
}

double value_get_float(const ValuePtr& val)
{
    return as_value(val).raw_get<Float>();
}

bool value_get_bool(const ValuePtr& val)
{
    return as_value(val).raw_get<Bool>();
}

const std::string& value_get_string(const ValuePtr& val)
{
    return as_value(val).raw_get<String>();
}

// ---- Stringification ----

std::unique_ptr<std::string> value_to_string(const ValuePtr& val)
{
    return std::make_unique<std::string>(as_value(val).to_internal_string());
}

std::unique_ptr<std::string> value_type_name(const ValuePtr& val)
{
    return std::make_unique<std::string>(as_value(val).type_name());
}

} // namespace frst::rs
