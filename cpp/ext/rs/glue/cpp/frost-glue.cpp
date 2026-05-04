#include "frost-glue.hpp"

#include <frost/builtin.hpp>
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

// ---- Collection factories ----

std::shared_ptr<const Value> value_from_array(
    rust::Slice<const std::shared_ptr<const Value>> elements)
{
    Array arr;
    arr.reserve(elements.size());
    for (const auto& elem : elements)
        arr.push_back(elem);
    return frst::Value::create(std::move(arr));
}

std::shared_ptr<const Value> value_from_map(
    rust::Slice<const std::shared_ptr<const Value>> keys,
    rust::Slice<const std::shared_ptr<const Value>> values)
{
    Map map;
    for (std::size_t i = 0; i < keys.size(); ++i)
        map.emplace(keys[i], values[i]);
    return frst::Value::create(std::move(map));
}

std::shared_ptr<const Value> value_from_map_trusted(
    rust::Slice<const std::shared_ptr<const Value>> keys,
    rust::Slice<const std::shared_ptr<const Value>> values)
{
    Map map;
    for (std::size_t i = 0; i < keys.size(); ++i)
        map.emplace(keys[i], values[i]);
    return frst::Value::create(frst::Value::trusted, std::move(map));
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

// ---- Category checks ----

bool value_is_numeric(const Value& val)
{
    return val.is_numeric();
}

bool value_is_primitive(const Value& val)
{
    return val.is_primitive();
}

bool value_is_structured(const Value& val)
{
    return val.is_structured();
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
        throw Frost_Recoverable_Error{"Array index out of bounds"};
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

std::shared_ptr<const Value> value_map_get_by(
    const Value& map_val, const std::shared_ptr<const Value>& key)
{
    const auto& map = map_val.raw_get<Map>();
    auto it = map.find(key);
    if (it == map.end())
        return frst::Value::null();
    return it->second;
}

bool value_map_contains_key(const Value& map_val,
                            const std::shared_ptr<const Value>& key)
{
    return map_val.raw_get<Map>().contains(key);
}

rust::Slice<const std::shared_ptr<const Value>>
value_map_keys(const Value& val)
{
    const auto& keys = val.raw_get<Map>().keys();
    return {keys.data(), keys.size()};
}

rust::Slice<const std::shared_ptr<const Value>>
value_map_values(const Value& val)
{
    const auto& values = val.raw_get<Map>().values();
    return {values.data(), values.size()};
}

// ---- Function call ----

std::shared_ptr<const Value> value_call(
    const Value& callable,
    rust::Slice<const std::shared_ptr<const Value>> args)
{
    const auto& fn = callable.raw_get<Function>();
    return fn->call({args.data(), args.size()});
}

// ---- Closure creation ----

Value_Ptr rust_closure_call(
    RustClosure const& closure,
    rust::Slice<Value_Ptr const> args);

std::shared_ptr<const Value> make_closure(rust::Box<RustClosure> closure)
{
    auto shared =
        std::make_shared<rust::Box<RustClosure>>(std::move(closure));
    return frst::Value::create(
        Function{std::make_shared<Builtin>(
            [shared](builtin_args_t args) -> Value_Ptr {
                try
                {
                    return rust_closure_call(**shared,
                                            {args.data(), args.size()});
                }
                catch (const rust::Error& e)
                {
                    throw Frost_Recoverable_Error{std::string(e.what())};
                }
            },
            "<rust closure>")});
}

// ---- Arithmetic ----

std::shared_ptr<const Value> value_add(
    const std::shared_ptr<const Value>& lhs,
    const std::shared_ptr<const Value>& rhs)
{
    return frst::Value::add(lhs, rhs);
}

std::shared_ptr<const Value> value_subtract(
    const std::shared_ptr<const Value>& lhs,
    const std::shared_ptr<const Value>& rhs)
{
    return frst::Value::subtract(lhs, rhs);
}

std::shared_ptr<const Value> value_multiply(
    const std::shared_ptr<const Value>& lhs,
    const std::shared_ptr<const Value>& rhs)
{
    return frst::Value::multiply(lhs, rhs);
}

std::shared_ptr<const Value> value_divide(
    const std::shared_ptr<const Value>& lhs,
    const std::shared_ptr<const Value>& rhs)
{
    return frst::Value::divide(lhs, rhs);
}

std::shared_ptr<const Value> value_modulus(
    const std::shared_ptr<const Value>& lhs,
    const std::shared_ptr<const Value>& rhs)
{
    return frst::Value::modulus(lhs, rhs);
}

// ---- Comparison ----

std::shared_ptr<const Value> value_equal(
    const std::shared_ptr<const Value>& lhs,
    const std::shared_ptr<const Value>& rhs)
{
    return frst::Value::equal(lhs, rhs);
}

std::shared_ptr<const Value> value_not_equal(
    const std::shared_ptr<const Value>& lhs,
    const std::shared_ptr<const Value>& rhs)
{
    return frst::Value::not_equal(lhs, rhs);
}

std::shared_ptr<const Value> value_less_than(
    const std::shared_ptr<const Value>& lhs,
    const std::shared_ptr<const Value>& rhs)
{
    return frst::Value::less_than(lhs, rhs);
}

std::shared_ptr<const Value> value_less_than_or_equal(
    const std::shared_ptr<const Value>& lhs,
    const std::shared_ptr<const Value>& rhs)
{
    return frst::Value::less_than_or_equal(lhs, rhs);
}

std::shared_ptr<const Value> value_greater_than(
    const std::shared_ptr<const Value>& lhs,
    const std::shared_ptr<const Value>& rhs)
{
    return frst::Value::greater_than(lhs, rhs);
}

std::shared_ptr<const Value> value_greater_than_or_equal(
    const std::shared_ptr<const Value>& lhs,
    const std::shared_ptr<const Value>& rhs)
{
    return frst::Value::greater_than_or_equal(lhs, rhs);
}

// ---- Coercion ----

bool value_truthy(const Value& val)
{
    return val.truthy();
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
