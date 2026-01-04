#ifndef FROST_VALUE_HPP
#define FROST_VALUE_HPP

#include <cassert>
#include <concepts>
#include <cstdint>
#include <map>
#include <memory>
#include <optional>
#include <string>
#include <variant>
#include <vector>

namespace frst
{

// ==========================================
// Basic type definitions
// ==========================================

class Value;
using Value_Ptr = std::shared_ptr<const Value>;

namespace impl
{
//! An implementation of less-than ONLY for comparing keys in a map
struct Value_Ptr_Less
{
    static bool operator()(const Value_Ptr& lhs, const Value_Ptr& rhs);
};
} // namespace impl

struct Null
{
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

using Map = std::map<Value_Ptr, Value_Ptr, impl::Value_Ptr_Less>;

// TODO: Closure

struct Frost_Error : std::runtime_error
{
    Frost_Error(const auto& err)
        : std::runtime_error{err}
    {
    }
};

inline namespace literals
{
consteval frst::Int operator""_f(unsigned long long val)
{
    return val;
}
} // namespace literals

template <typename T>
concept Frost_Type =
    std::same_as<Null, T> || std::same_as<Int, T> || std::same_as<Float, T> ||
    std::same_as<Bool, T> || std::same_as<String, T> ||
    std::same_as<Array, T> || std::same_as<Map, T>;

// ==========================================
// Type name table
// ==========================================

#define VALUE_STRINGIZE_IMPL(X) #X
#define VALUE_STRINGIZE(X) VALUE_STRINGIZE_IMPL(X)

template <Frost_Type T>
std::string_view type_str() = delete;

#define TYPE_STR_SPEC(T)                                                       \
    template <>                                                                \
    inline std::string_view type_str<T>()                                      \
    {                                                                          \
        return VALUE_STRINGIZE(T);                                             \
    }

TYPE_STR_SPEC(Null)
TYPE_STR_SPEC(Int)
TYPE_STR_SPEC(Float)
TYPE_STR_SPEC(String)
TYPE_STR_SPEC(Bool)
TYPE_STR_SPEC(Array)
TYPE_STR_SPEC(Map)

#undef TYPE_STR_SPEC
#undef STRINGIZE

struct Type_Str_Fn
{
    template <Frost_Type T>
    static std::string_view operator()(const T&)
    {
        return type_str<T>();
    }
} constexpr inline type_str_niebloid;

// ==========================================
// Type coercion tables
// ==========================================

template <Frost_Type T>
struct coerce_to;

#define COERCIONS_TO(Target)                                                   \
    template <>                                                                \
    struct coerce_to<Target>                                                   \
    {                                                                          \
        using target_t = Target;                                               \
        void completeness_check()                                              \
        {                                                                      \
            (*this)(Null{});                                                   \
            (*this)(Int{});                                                    \
            (*this)(Float{});                                                  \
            (*this)(Bool{});                                                   \
            (*this)(String{});                                                 \
            (*this)(Array{});                                                  \
            (*this)(Map{});                                                    \
        }

#define COERCE(From)                                                           \
    static std::optional<target_t> operator()(                                 \
        [[maybe_unused]] const From& value)

#define NO_COERCE(From)                                                        \
    COERCE(From)                                                               \
    {                                                                          \
        return std::nullopt;                                                   \
    }

#define VALUE_COERCE(From)                                                     \
    COERCE(From)                                                               \
    {                                                                          \
        return value;                                                          \
    }

#define END_COERCIONS                                                          \
    }                                                                          \
    ;

COERCIONS_TO(Null)
VALUE_COERCE(Null)
NO_COERCE(Int)
NO_COERCE(Float)
NO_COERCE(Bool)
NO_COERCE(String)
NO_COERCE(Array)
NO_COERCE(Map)
END_COERCIONS

COERCIONS_TO(Int)
NO_COERCE(Null)
VALUE_COERCE(Int)
VALUE_COERCE(Float)
NO_COERCE(Bool)
NO_COERCE(String)
NO_COERCE(Array)
NO_COERCE(Map)
END_COERCIONS

COERCIONS_TO(Float)
NO_COERCE(Null)
VALUE_COERCE(Int)
VALUE_COERCE(Float)
NO_COERCE(Bool)
NO_COERCE(String)
NO_COERCE(Array)
NO_COERCE(Map)
END_COERCIONS

COERCIONS_TO(Bool)

COERCE(Null)
{
    return false;
}

COERCE(Int)
{
    return true;
}

COERCE(Float)
{
    return true;
}

VALUE_COERCE(Bool)

COERCE(String)
{
    return true;
}

COERCE(Array)
{
    return true;
}

COERCE(Map)
{
    return true;
}

END_COERCIONS

COERCIONS_TO(String)
NO_COERCE(Null)
NO_COERCE(Int)
NO_COERCE(Float)
NO_COERCE(Bool)
VALUE_COERCE(String)
NO_COERCE(Array)
NO_COERCE(Map)
END_COERCIONS

COERCIONS_TO(Array)
NO_COERCE(Null)
NO_COERCE(Int)
NO_COERCE(Float)
NO_COERCE(Bool)
NO_COERCE(String)
VALUE_COERCE(Array)
NO_COERCE(Map)
END_COERCIONS

COERCIONS_TO(Map)
NO_COERCE(Null)
NO_COERCE(Int)
NO_COERCE(Float)
NO_COERCE(Bool)
NO_COERCE(String)
NO_COERCE(Array)
VALUE_COERCE(Map)
END_COERCIONS

#undef COERSIONS_TO
#undef COERCE
#undef NO_COERCE
#undef END_COERCIONS

// ==========================================
// The actual Value class
// ==========================================

class Value
{
  public:
    using Ptr = Value_Ptr;
    friend impl::Value_Ptr_Less;

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

    template <typename... Args>
    static Ptr create(Args&&... args)
    {
        return std::make_shared<Value>(std::forward<Args>(args)...);
    }

    //! @brief Check if a Value is a particular type
    template <Frost_Type T>
    bool is() const
    {
        return std::holds_alternative<T>(value_);
    }

    bool is_numeric() const
    {
        return is<Int>() || is<Float>();
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
    template <Frost_Type T>
    std::optional<T> as() const
    {
        return value_.visit(coerce_to<T>{});
    }

    std::string_view type_name() const
    {
        return value_.visit(type_str_niebloid);
    }

    static Value_Ptr add(const Value_Ptr& lhs, const Value_Ptr& rhs);
    static Value_Ptr subtract(const Value_Ptr& lhs, const Value_Ptr& rhs);
    static Value_Ptr multiply(const Value_Ptr& lhs, const Value_Ptr& rhs);
    static Value_Ptr divide(const Value_Ptr& lhs, const Value_Ptr& rhs);

  private:
    std::variant<Null, Int, Float, Bool, String, Array, Map> value_;
};

#define STRINGIZE(X) #X
#define THROW_UNREACHABLE                                                      \
    throw Frost_Error                                                          \
    {                                                                          \
        "Hit point which should be unreachable at: " __FILE__                  \
        ":" STRINGIZE(__LINE__)                                                          \
    }

inline bool impl::Value_Ptr_Less::operator()(const Value_Ptr& lhs,
                                             const Value_Ptr& rhs)
{
    return lhs->value_ < rhs->value_;
}

} // namespace frst

#endif
