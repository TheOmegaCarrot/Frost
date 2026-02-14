#ifndef FROST_TYPES_HPP
#define FROST_TYPES_HPP

#include "value-fwd.hpp"

#include <concepts>
#include <cstdint>
#include <flat_map>
#include <span>
#include <vector>

namespace frst
{

// ==========================================
// Basic type definitions
// ==========================================

struct Null
{
    friend bool operator==(Null, Null)
    {
        return true;
    }

    friend bool operator<(Null, Null)
    {
        return false;
    }
};

using Int = std::int64_t;

using Float = double;

using Bool = bool;

using String = std::string;

using Array = std::vector<Value_Ptr>;

using Map = std::flat_map<Value_Ptr, Value_Ptr, impl::Value_Ptr_Less>;

class Callable
{
  public:
    virtual ~Callable() = default;

    virtual Value_Ptr call(std::span<const Value_Ptr> args) const = 0;
    virtual std::string debug_dump() const = 0;
};

using Function = std::shared_ptr<Callable>;

inline namespace literals
{
consteval frst::Int operator""_f(unsigned long long val)
{
    return val;
}
} // namespace literals

template <typename T>
concept Frost_Numeric = std::same_as<Int, T> || std::same_as<Float, T>;

template <typename T>
concept Frost_Primitive = Frost_Numeric<T>
                          || std::same_as<Null, T>
                          || std::same_as<Bool, T>
                          || std::same_as<String, T>;

template <typename T>
concept Frost_Structured = std::same_as<Array, T> || std::same_as<Map, T>;

template <typename T>
concept Frost_Type =
    Frost_Primitive<T> || Frost_Structured<T> || std::same_as<Function, T>;

} // namespace frst

#endif
