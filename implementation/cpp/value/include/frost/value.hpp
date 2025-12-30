#ifndef FROST_VALUE_HPP
#define FROST_VALUE_HPP

#include <cassert>
#include <concepts>
#include <cstdint>
#include <memory>
#include <optional>
#include <string>
#include <unordered_map>
#include <variant>
#include <vector>

namespace frst
{

// ==========================================
// Basic type definitions
// ==========================================

class Value;
using Value_Ptr = std::shared_ptr<const Value>;

struct Null
{
};

struct Int_Tag
{
};
using Int = std::int64_t;

struct Float_Tag
{
};
using Float = double;

struct Bool_Tag
{
};
using Bool = bool;

struct String_Tag
{
};
using String = std::string;

using Array = std::vector<Value_Ptr>;

using Map = std::unordered_map<Value_Ptr, Value_Ptr>;

// TODO: Closure

template <typename T>
concept Frost_Type =
    std::same_as<Null, T> || std::same_as<Int, T> || std::same_as<Float, T> ||
    std::same_as<Bool, T> || std::same_as<String, T> ||
    std::same_as<Array, T> || std::same_as<Map, T>;

// ==========================================
// Type coercion tables
// ==========================================

template <Frost_Type T>
struct coerce_to;

#define COERCE(T)                                                              \
    static std::optional<T> operator()([[maybe_unused]] const T& value)

template <>
struct coerce_to<Null>
{
    template <Frost_Type T>
    static std::optional<Null> operator()(const T&)
    {
        assert(false && "Attempting to coerce to null!");
    }
};

template <>
struct coerce_to<Int>
{
    COERCE(Null)
    {
        return std::nullopt;
    }

    COERCE(Int)
    {
        return value;
    }

    COERCE(Float)
    {
        return value; // use builtin C++ truncation
    }

    COERCE(String)
    {
        return std::nullopt;
    }

    COERCE(Array)
    {
        return std::nullopt;
    }

    COERCE(Map)
    {
        return std::nullopt;
    }
};

template <>
struct coerce_to<Float>
{
};

#undef COERCE

class Value
{
  public:
    using Ptr = Value_Ptr;

    Value()
        : value_{Null{}}
    {
    }

    Value(const Value&) = default;
    Value(Value&&) = default;
    Value& operator=(const Value&) = default;
    Value& operator=(Value&&) = default;
    ~Value() = default;

    template <Frost_Type T>
    Value(T&& value)
        : value_{std::move(value)}
    {
    }

    //! @brief Check if a Value is a particular type
    template <Frost_Type T>
    bool is() const
    {
        return std::holds_alternative<T>(value_);
    }

    //! @brief Get the contained value by exact type
    template <Frost_Type T>
    std::optional<T> get() const
    {
        if (is<T>())
            return std::get<T>(value_);
        else
            return std::nullopt;
    }

    //! @brief Attempt to coerce the Value to a particular type
    // template <Frost_Type T>
    // std::optional<T> as()
    // {
    //     return value_.visit(coerce_to<T>{});
    // }

  private:
    std::variant<Null, Int, Float, Bool, String, Array, Map> value_;
};

} // namespace frst

#endif
