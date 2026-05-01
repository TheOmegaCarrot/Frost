#ifndef FROST_RS_GLUE_HPP
#define FROST_RS_GLUE_HPP

// Pure C++ declarations -- no frost headers, no cxx headers.
// cxx-build can compile the generated bridge against this without needing
// the frost include tree. Implementations live in frost-glue.cpp.

#include <cstdint>
#include <memory>
#include <string>

namespace frst
{
class Value;
} // namespace frst

namespace frst::rs
{

// Same type as Value_Ptr (shared_ptr<const Value>), so no casts needed
// when passing between shim code and the Frost runtime.
using Value = const frst::Value;

// ---- Factories ----
std::shared_ptr<const Value> value_null();
std::shared_ptr<const Value> value_from_int(int64_t val);
std::shared_ptr<const Value> value_from_float(double val);
std::shared_ptr<const Value> value_from_bool(bool val);
std::shared_ptr<const Value> value_from_string(const std::string& val);

// ---- Type checks ----
bool value_is_null(const Value& val);
bool value_is_int(const Value& val);
bool value_is_float(const Value& val);
bool value_is_bool(const Value& val);
bool value_is_string(const Value& val);
bool value_is_array(const Value& val);
bool value_is_map(const Value& val);
bool value_is_function(const Value& val);

// ---- Accessors (caller must type-check first) ----
int64_t value_get_int(const Value& val);
double value_get_float(const Value& val);
bool value_get_bool(const Value& val);
const std::string& value_get_string(const Value& val);

// ---- Stringification ----
std::unique_ptr<std::string> value_to_string(const Value& val);
std::unique_ptr<std::string> value_type_name(const Value& val);

} // namespace frst::rs

#endif
