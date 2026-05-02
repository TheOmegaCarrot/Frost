#ifndef FROST_RS_GLUE_HPP
#define FROST_RS_GLUE_HPP

// Pure C++ declarations -- no frost headers, no cxx headers.
// cxx-build can compile the generated bridge against this without needing
// the frost include tree. Implementations live in frost-glue.cpp.

#include <cstdint>
#include <memory>
#include <string>

#include <rust/cxx.h>

namespace frst
{
class Value;
} // namespace frst

namespace frst::rs
{

// Same type as Value_Ptr (shared_ptr<const Value>), so no casts needed
// when passing between shim code and the Frost runtime.
using Value = const frst::Value;

// ---- Primitive factories ----
std::shared_ptr<const Value> value_null();
std::shared_ptr<const Value> value_from_int(int64_t val);
std::shared_ptr<const Value> value_from_float(double val);
std::shared_ptr<const Value> value_from_bool(bool val);
std::shared_ptr<const Value> value_from_string(const std::string& val);

// ---- Collection factories ----
std::shared_ptr<const Value> value_from_array(
    rust::Slice<const std::shared_ptr<const Value>> elements);
std::shared_ptr<const Value> value_from_map(
    rust::Slice<const std::shared_ptr<const Value>> keys,
    rust::Slice<const std::shared_ptr<const Value>> values);
std::shared_ptr<const Value> value_from_map_trusted(
    rust::Slice<const std::shared_ptr<const Value>> keys,
    rust::Slice<const std::shared_ptr<const Value>> values);

// ---- Type checks ----
bool value_is_null(const Value& val);
bool value_is_int(const Value& val);
bool value_is_float(const Value& val);
bool value_is_bool(const Value& val);
bool value_is_string(const Value& val);
bool value_is_array(const Value& val);
bool value_is_map(const Value& val);
bool value_is_function(const Value& val);

// ---- Primitive accessors (caller must type-check first) ----
int64_t value_get_int(const Value& val);
double value_get_float(const Value& val);
bool value_get_bool(const Value& val);
const std::string& value_get_string(const Value& val);

// ---- Array accessors (caller must verify is_array first) ----
size_t value_array_len(const Value& val);
std::shared_ptr<const Value> value_array_get(const Value& val, size_t index);
rust::Slice<const std::shared_ptr<const Value>> value_array_slice(
    const Value& val);

// ---- Map accessors (caller must verify is_map first) ----
size_t value_map_len(const Value& val);
bool value_map_has(const Value& val, const std::string& key);
std::shared_ptr<const Value> value_map_get(const Value& val,
                                           const std::string& key);
std::shared_ptr<const Value> value_map_get_by(
    const Value& map, const std::shared_ptr<const Value>& key);
bool value_map_contains_key(
    const Value& map, const std::shared_ptr<const Value>& key);
rust::Slice<const std::shared_ptr<const Value>> value_map_keys(
    const Value& val);
rust::Slice<const std::shared_ptr<const Value>> value_map_values(
    const Value& val);

// ---- Function call (caller must verify is_function first) ----
// Frost exceptions (std::exception subclasses) are caught by cxx and
// mapped to Result::Err on the Rust side.
std::shared_ptr<const Value> value_call(
    const Value& callable,
    rust::Slice<const std::shared_ptr<const Value>> args);

// ---- Closure creation ----
// Wraps a Rust closure in a Frost Builtin. Rust Err maps to
// Frost_Recoverable_Error via rust::Error catch.
struct RustClosure;
std::shared_ptr<const Value> make_closure(rust::Box<RustClosure> closure);

// ---- Arithmetic (throw on type mismatch) ----
std::shared_ptr<const Value> value_add(
    const std::shared_ptr<const Value>& lhs,
    const std::shared_ptr<const Value>& rhs);
std::shared_ptr<const Value> value_subtract(
    const std::shared_ptr<const Value>& lhs,
    const std::shared_ptr<const Value>& rhs);
std::shared_ptr<const Value> value_multiply(
    const std::shared_ptr<const Value>& lhs,
    const std::shared_ptr<const Value>& rhs);
std::shared_ptr<const Value> value_divide(
    const std::shared_ptr<const Value>& lhs,
    const std::shared_ptr<const Value>& rhs);
std::shared_ptr<const Value> value_modulus(
    const std::shared_ptr<const Value>& lhs,
    const std::shared_ptr<const Value>& rhs);

// ---- Comparison ----
std::shared_ptr<const Value> value_equal(
    const std::shared_ptr<const Value>& lhs,
    const std::shared_ptr<const Value>& rhs);
std::shared_ptr<const Value> value_not_equal(
    const std::shared_ptr<const Value>& lhs,
    const std::shared_ptr<const Value>& rhs);
std::shared_ptr<const Value> value_less_than(
    const std::shared_ptr<const Value>& lhs,
    const std::shared_ptr<const Value>& rhs);
std::shared_ptr<const Value> value_less_than_or_equal(
    const std::shared_ptr<const Value>& lhs,
    const std::shared_ptr<const Value>& rhs);
std::shared_ptr<const Value> value_greater_than(
    const std::shared_ptr<const Value>& lhs,
    const std::shared_ptr<const Value>& rhs);
std::shared_ptr<const Value> value_greater_than_or_equal(
    const std::shared_ptr<const Value>& lhs,
    const std::shared_ptr<const Value>& rhs);

// ---- Coercion ----
bool value_truthy(const Value& val);

// ---- Stringification ----
std::unique_ptr<std::string> value_to_string(const Value& val);
std::unique_ptr<std::string> value_type_name(const Value& val);

} // namespace frst::rs

#endif
