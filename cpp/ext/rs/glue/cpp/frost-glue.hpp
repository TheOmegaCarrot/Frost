#ifndef FROST_RS_GLUE_HPP
#define FROST_RS_GLUE_HPP

// Pure C++ declarations -- no frost headers, no cxx headers.
// cxx-build can compile the generated bridge against this without needing
// the frost include tree. Implementations live in frost-glue.cpp.

#include <cstdint>
#include <memory>
#include <string>

namespace frst::rs
{

class ValuePtr;

// ---- Factories ----
std::shared_ptr<ValuePtr> value_null();
std::shared_ptr<ValuePtr> value_from_int(int64_t val);
std::shared_ptr<ValuePtr> value_from_float(double val);
std::shared_ptr<ValuePtr> value_from_bool(bool val);
std::shared_ptr<ValuePtr> value_from_string(const std::string& val);

// ---- Type checks ----
bool value_is_null(const ValuePtr& val);
bool value_is_int(const ValuePtr& val);
bool value_is_float(const ValuePtr& val);
bool value_is_bool(const ValuePtr& val);
bool value_is_string(const ValuePtr& val);
bool value_is_array(const ValuePtr& val);
bool value_is_map(const ValuePtr& val);
bool value_is_function(const ValuePtr& val);

// ---- Accessors (caller must type-check first) ----
int64_t value_get_int(const ValuePtr& val);
double value_get_float(const ValuePtr& val);
bool value_get_bool(const ValuePtr& val);
const std::string& value_get_string(const ValuePtr& val);

// ---- Stringification ----
std::unique_ptr<std::string> value_to_string(const ValuePtr& val);
std::unique_ptr<std::string> value_type_name(const ValuePtr& val);

} // namespace frst::rs

#endif
