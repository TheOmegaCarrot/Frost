#ifndef FROST_VALUE_HPP
#define FROST_VALUE_HPP

#include <cassert>
#include <concepts>
#include <cstdint>
#include <limits>
#include <map>
#include <memory>
#include <optional>
#include <span>
#include <string>
#include <variant>
#include <vector>

#include <fmt/format.h>

#include "exceptions.hpp"
#include "value-fwd.hpp"

#include <boost/preprocessor/stringize.hpp>

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

// Idea: replace structured types with classes which allow for computed values
// e.g. `1..10` -> "An array" without actually creating the Values

using Array = std::vector<Value_Ptr>;

using Map = std::map<Value_Ptr, Value_Ptr, impl::Value_Ptr_Less>;

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
        return BOOST_PP_STRINGIZE(T);                                          \
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
} constexpr inline type_str_fn;

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
static std::optional<target_t> operator()([[maybe_unused]] const Float& value)
{
    if ((value < std::numeric_limits<Int>::lowest())
        || (value > static_cast<Float>(std::numeric_limits<Int>::max())))
        throw Frost_Recoverable_Error{
            fmt::format("Value {} is out of range of Int", value)};

    return value;
}
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

COERCIONS_TO(Function)
NO_COERCE(Null)
NO_COERCE(Int)
NO_COERCE(Float)
NO_COERCE(Bool)
NO_COERCE(String)
NO_COERCE(Array)
NO_COERCE(Map)
VALUE_COERCE(Function)
END_COERCIONS

#undef COERCIONS_TO
#undef COERCE
#undef VALUE_COERCE
#undef NO_COERCE
#undef END_COERCIONS

struct To_Internal_String_Params
{
    bool in_structure = false;
    bool pretty = false;
    std::size_t depth = 0;
};

// ==========================================
// The actual Value class
// ==========================================

class Value
{
    // Special tag for constructing deduplication singletons
    struct Singleton_Tag
    {
    };

  public:
    using Ptr = Value_Ptr;
    friend impl::Value_Ptr_Less;

    Value() = delete;

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

    Value(Null) = delete;
    Value(Bool) = delete;

    Value(int val)
        : value_{val}
    {
    }

    Value(Int val)
        : value_{val}
    {
    }

    Value(Float val)
        : value_{val}
    {
        if (std::isnan(val))
            throw Frost_Recoverable_Error{
                "Floating-point computation produced NaN"};
        if (std::isinf(val))
            throw Frost_Recoverable_Error{
                "Floating-point computation produced infinity"};
    }

    Value(Singleton_Tag, Null)
        : value_{Null{}}
    {
    }

    Value(Singleton_Tag, Bool b)
        : value_{b}
    {
    }

    static Value_Ptr null()
    {
        return null_singleton_;
    }

    template <typename... Args>
    [[nodiscard]] static Value_Ptr create(Args&&... args)
    {
        return std::make_shared<Value>(std::forward<Args>(args)...);
    }

    [[nodiscard]] static Value_Ptr create()
    {
        return null_singleton_;
    }

    [[nodiscard]] static Value_Ptr create(Null)
    {
        return null_singleton_;
    }

    [[nodiscard]] static Value_Ptr create(Bool b)
    {
        if (b)
            return true_singleton_;
        else
            return false_singleton_;
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
            [](const Frost_Numeric auto&) {
                return true;
            },
            [](const auto&) {
                return false;
            },
        });
    }

    [[nodiscard]] bool is_primitive() const
    {
        return value_.visit(Overload{
            [](const Frost_Primitive auto&) {
                return true;
            },
            [](const auto&) {
                return false;
            },
        });
    }

    [[nodiscard]] bool is_structured() const
    {
        return value_.visit(Overload{
            [](const Frost_Structured auto&) {
                return true;
            },
            [](const auto&) {
                return false;
            },
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

    [[nodiscard]] bool truthy() const
    {
        return as<Bool>().value();
    }

    [[nodiscard]] std::string_view type_name() const
    {
        return value_.visit(type_str_fn);
    }

    Value_Ptr clone() const;

    // Convert the value to a string as for user-facing output
    [[nodiscard]] std::string to_internal_string(
        To_Internal_String_Params params = {}) const;

    // The user-facing to_string function
    [[nodiscard]] Value_Ptr to_string() const
    {
        return create(to_internal_string());
    }

    // The user-facing to_string function
    [[nodiscard]] Value_Ptr to_pretty_string() const
    {
        return create(to_internal_string({.pretty = true}));
    }

    [[nodiscard]] std::optional<Int> to_internal_int() const;

    [[nodiscard]] Value_Ptr to_int() const;

    [[nodiscard]] std::optional<Float> to_internal_float() const;

    [[nodiscard]] Value_Ptr to_float() const;

    // unary -
    [[nodiscard]] Value_Ptr negate() const;

    static Value_Ptr add(const Value_Ptr& lhs, const Value_Ptr& rhs);
    static Value_Ptr subtract(const Value_Ptr& lhs, const Value_Ptr& rhs);
    static Value_Ptr multiply(const Value_Ptr& lhs, const Value_Ptr& rhs);
    static Value_Ptr divide(const Value_Ptr& lhs, const Value_Ptr& rhs);
    static Value_Ptr modulus(const Value_Ptr& lhs, const Value_Ptr& rhs);

    static Value_Ptr deep_equal(const Value_Ptr& lhs, const Value_Ptr& rhs)
    {
        return create(deep_equal_impl(lhs, rhs));
    }

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
        return Value::create(not truthy());
    }

    // Shared implementation between AST nodes and some builtins
    // Requires type-checking of structure before calling
    static Value_Ptr do_map(Value_Ptr structure, const Function& fn,
                            std::string_view parent_op_name);
    static Value_Ptr do_filter(Value_Ptr structure, const Function& fn);
    static Value_Ptr do_reduce(Value_Ptr structure, const Function& fn,
                               std::optional<Value_Ptr> init);

    decltype(auto) visit(this auto&& self, auto&& visitor)
    {
        return std::forward<decltype(self)>(self).value_.visit(
            std::forward<decltype(visitor)>(visitor));
    }

  private:
    std::variant<Null, Int, Float, Bool, String, Array, Map, Function> value_;

    static bool deep_equal_impl(const Value_Ptr& lhs, const Value_Ptr& rhs);

    static bool equal_impl(const Value_Ptr& lhs, const Value_Ptr& rhs);
    static bool not_equal_impl(const Value_Ptr& lhs, const Value_Ptr& rhs);
    static bool less_than_impl(const Value_Ptr& lhs, const Value_Ptr& rhs);
    static bool less_than_or_equal_impl(const Value_Ptr& lhs,
                                        const Value_Ptr& rhs);
    static bool greater_than_impl(const Value_Ptr& lhs, const Value_Ptr& rhs);
    static bool greater_than_or_equal_impl(const Value_Ptr& lhs,
                                           const Value_Ptr& rhs);

    static inline Value_Ptr null_singleton_ =
        std::make_shared<Value>(Singleton_Tag{}, Null{});
    static inline Value_Ptr true_singleton_ =
        std::make_shared<Value>(Singleton_Tag{}, true);
    static inline Value_Ptr false_singleton_ =
        std::make_shared<Value>(Singleton_Tag{}, false);
};

inline namespace literals
{
inline Value_Ptr operator""_s(const char* str, std::size_t)
{
    return Value::create(std::string{str});
}
} // namespace literals

} // namespace frst

#endif
