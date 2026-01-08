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

// Idea: replace structured types with classes which allow for computed values
// e.g. `1..10` -> "An array" without actually creating the Values

using Array = std::vector<Value_Ptr>;

using Map = std::map<Value_Ptr, Value_Ptr, impl::Value_Ptr_Less>;

class Symbol_Table;
class Callable
{
  public:
    virtual ~Callable() = default;

    virtual Value_Ptr call(const std::vector<Value_Ptr>& args) const = 0;
    virtual std::string debug_dump() const = 0;
};

using Function = std::shared_ptr<Callable>;

struct Frost_Error : std::runtime_error
{
    Frost_Error(const auto& err)
        : std::runtime_error{err}
    {
    }
};

#define VALUE_STRINGIZE_IMPL(X) #X
#define VALUE_STRINGIZE(X) VALUE_STRINGIZE_IMPL(X)

#define THROW_UNREACHABLE                                                      \
    throw Frost_Error                                                          \
    {                                                                          \
        "Hit point which should be unreachable at: " __FILE__                  \
        ":" VALUE_STRINGIZE(__LINE__)                                          \
    }

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
concept Frost_Primitive = Frost_Numeric<T> || std::same_as<Null, T> ||
                          std::same_as<Bool, T> || std::same_as<String, T>;

template <typename T>
concept Frost_Structured = std::same_as<Array, T> || std::same_as<Map, T>;

template <typename T>
concept Frost_Type =
    Frost_Primitive<T> || Frost_Structured<T> || std::same_as<Function, T>;

template <typename... Ls>
struct Overload : Ls...
{
    using Ls::operator()...;
};

// ==========================================
// Type name table
// ==========================================

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
TYPE_STR_SPEC(Function)

#undef TYPE_STR_SPEC

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
            (*this)(Function{});                                               \
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
NO_COERCE(Function)
END_COERCIONS

COERCIONS_TO(Int)
NO_COERCE(Null)
VALUE_COERCE(Int)
VALUE_COERCE(Float)
NO_COERCE(Bool)
NO_COERCE(String)
NO_COERCE(Array)
NO_COERCE(Map)
NO_COERCE(Function)
END_COERCIONS

COERCIONS_TO(Float)
NO_COERCE(Null)
VALUE_COERCE(Int)
VALUE_COERCE(Float)
NO_COERCE(Bool)
NO_COERCE(String)
NO_COERCE(Array)
NO_COERCE(Map)
NO_COERCE(Function)
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

COERCE(Function)
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
NO_COERCE(Function)
END_COERCIONS

COERCIONS_TO(Array)
NO_COERCE(Null)
NO_COERCE(Int)
NO_COERCE(Float)
NO_COERCE(Bool)
NO_COERCE(String)
VALUE_COERCE(Array)
NO_COERCE(Map)
NO_COERCE(Function)
END_COERCIONS

COERCIONS_TO(Map)
NO_COERCE(Null)
NO_COERCE(Int)
NO_COERCE(Float)
NO_COERCE(Bool)
NO_COERCE(String)
NO_COERCE(Array)
VALUE_COERCE(Map)
NO_COERCE(Function)
END_COERCIONS

#undef COERCIONS_TO
#undef COERCE
#undef VALUE_COERCE
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

    Value(const Value&) = delete;
    Value(Value&&) = default;
    Value& operator=(const Value&) = delete;
    Value& operator=(Value&&) = default;
    ~Value() = default;

    template <Frost_Type T>
    Value(T&& value)
        : value_{std::move(value)}
    {
    }

    template <typename... Args>
    [[nodiscard]] static Ptr create(Args&&... args)
    {
        return std::make_shared<Value>(std::forward<Args>(args)...);
    }

    //! @brief Check if a Value is a particular type
    template <Frost_Type T>
    [[nodiscard]] bool is() const
    {
        return std::holds_alternative<T>(value_);
    }

    [[nodiscard]] bool is_numeric() const
    {
        return value_.visit(Overload{
            [](const Frost_Numeric auto&) { return true; },
            [](const auto&) { return false; },
        });
    }

    [[nodiscard]] bool is_primitive() const
    {
        return value_.visit(Overload{
            [](const Frost_Primitive auto&) { return true; },
            [](const auto&) { return false; },
        });
    }

    [[nodiscard]] bool is_structured() const
    {
        return value_.visit(Overload{
            [](const Frost_Structured auto&) { return true; },
            [](const auto&) { return false; },
        });
    }

    //! @brief Get the contained value by exact type
    template <Frost_Type T>
    [[nodiscard]] std::optional<T> get() const
    {
        if (is<T>())
            return std::get<T>(value_);
        else
            return std::nullopt;
    }

    //! @brief CAREFULLY get a reference
    //! (optional<T&> would make this unnecessary)
    template <Frost_Type T>
    [[nodiscard]] const T& raw_get() const
    {
        if (is<T>())
            return std::get<T>(value_);
        else
            THROW_UNREACHABLE;
    }

    //! @brief Attempt to coerce the Value to a particular type
    template <Frost_Type T>
    [[nodiscard]] std::optional<T> as() const
    {
        return value_.visit(coerce_to<T>{});
    }

    [[nodiscard]] std::string_view type_name() const
    {
        return value_.visit(type_str_niebloid);
    }

    // Convert the value to a string as for user-facing output
    std::string to_internal_string(bool in_structure = false) const;

    // The user-facing to_string function
    [[nodiscard]] Value_Ptr to_string() const
    {
        return create(to_internal_string());
    }

    // unary -
    [[nodiscard]] Value_Ptr negate() const;

    static Value_Ptr add(const Value_Ptr& lhs, const Value_Ptr& rhs);
    static Value_Ptr subtract(const Value_Ptr& lhs, const Value_Ptr& rhs);
    static Value_Ptr multiply(const Value_Ptr& lhs, const Value_Ptr& rhs);
    static Value_Ptr divide(const Value_Ptr& lhs, const Value_Ptr& rhs);

    static Value_Ptr equal(const Value_Ptr& lhs, const Value_Ptr& rhs)
    {
        return create(equal_impl(lhs, rhs));
    }
    static Value_Ptr not_equal(const Value_Ptr& lhs, const Value_Ptr& rhs)
    {
        return create(not_equal_impl(lhs, rhs));
    }
    static Value_Ptr less_than(const Value_Ptr& lhs, const Value_Ptr& rhs)
    {
        return create(less_than_impl(lhs, rhs));
    }
    static Value_Ptr less_than_or_equal(const Value_Ptr& lhs,
                                        const Value_Ptr& rhs)
    {
        return create(less_than_or_equal_impl(lhs, rhs));
    }
    static Value_Ptr greater_than(const Value_Ptr& lhs, const Value_Ptr& rhs)
    {
        return create(greater_than_impl(lhs, rhs));
    }
    static Value_Ptr greater_than_or_equal(const Value_Ptr& lhs,
                                           const Value_Ptr& rhs)
    {
        return create(greater_than_or_equal_impl(lhs, rhs));
    }

    [[nodiscard]] Value_Ptr logical_not() const
    {
        return Value::create(not as<Bool>().value());
    }

  private:
    std::variant<Null, Int, Float, Bool, String, Array, Map, Function> value_;

    static bool equal_impl(const Value_Ptr& lhs, const Value_Ptr& rhs);
    static bool not_equal_impl(const Value_Ptr& lhs, const Value_Ptr& rhs);
    static bool less_than_impl(const Value_Ptr& lhs, const Value_Ptr& rhs);
    static bool less_than_or_equal_impl(const Value_Ptr& lhs,
                                        const Value_Ptr& rhs);
    static bool greater_than_impl(const Value_Ptr& lhs, const Value_Ptr& rhs);
    static bool greater_than_or_equal_impl(const Value_Ptr& lhs,
                                           const Value_Ptr& rhs);
};

inline bool impl::Value_Ptr_Less::operator()(const Value_Ptr& lhs,
                                             const Value_Ptr& rhs)
{
    const auto lhs_index = lhs->value_.index();
    const auto rhs_index = rhs->value_.index();

    if (lhs_index != rhs_index)
        return lhs_index < rhs_index;

    if (lhs->is_primitive())
    {
        return std::visit(
            Overload{
                []<Frost_Primitive T>(const T& a, const T& b) { return a < b; },
                [](const auto&, const auto&) -> bool { THROW_UNREACHABLE; },
            },
            lhs->value_, rhs->value_);
    }

    return lhs.get() < rhs.get();
}

} // namespace frst

#endif
