#include "frost-glue.hpp"

#include <frost/value.hpp>

#include <rust/cxx.h>

namespace frst::rs
{

// frst::rs::Value is const frst::Value (via the typedef in the header),
// so shared_ptr<const Value> is the same type as Value_Ptr.

// ---- Factories ----

std::shared_ptr<const Value> value_null()
{
    return frst::Value::null();
}

std::shared_ptr<const Value> value_from_int(int64_t val)
{
    return frst::Value::create(Int{val});
}

std::shared_ptr<const Value> value_from_float(double val)
{
    return frst::Value::create(Float{val});
}

std::shared_ptr<const Value> value_from_bool(bool val)
{
    return frst::Value::create(Bool{val});
}

std::shared_ptr<const Value> value_from_string(const std::string& val)
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

// ---- Array accessors ----

size_t value_array_len(const Value& val)
{
    return val.raw_get<Array>().size();
}

std::shared_ptr<const Value> value_array_get(const Value& val, size_t index)
{
    const auto& arr = val.raw_get<Array>();
    if (index >= arr.size())
        return frst::Value::null();
    return arr[index];
}

rust::Slice<const std::shared_ptr<const Value>>
value_array_slice(const Value& val)
{
    const auto& arr = val.raw_get<Array>();
    return {arr.data(), arr.size()};
}

// ---- Map accessors ----

size_t value_map_len(const Value& val)
{
    return val.raw_get<Map>().size();
}

bool value_map_has(const Value& val, const std::string& key)
{
    return val.raw_get<Map>().contains(frst::Value::create(String{key}));
}

std::shared_ptr<const Value> value_map_get(const Value& val,
                                           const std::string& key)
{
    const auto& map = val.raw_get<Map>();
    auto it = map.find(frst::Value::create(String{key}));
    if (it == map.end())
        return frst::Value::null();
    return it->second;
}

// ---- Function call ----

std::shared_ptr<const Value> value_call(
    const Value& callable,
    rust::Slice<const std::shared_ptr<const Value>> args)
{
    const auto& fn = callable.raw_get<Function>();
    return fn->call({args.data(), args.size()});
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
