#ifndef FROST_VALUE_HPP
#define FROST_VALUE_HPP

#include <cassert>
#include <memory>
#include <optional>
#include <string>
#include <variant>

#include <fmt/format.h>

#include <boost/preprocessor/stringize.hpp>

#include "coercion.hpp"
#include "exceptions.hpp"
#include "type-strings.hpp"
#include "types.hpp"
#include "value-fwd.hpp"

namespace frst
{

template <typename... Ls>
struct Overload : Ls...
{
    using Ls::operator()...;
};

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
    } constexpr static singleton_tag{};

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

    Value(Map&& map)
    {
        for (const auto& [key, _] : map)
        {
            if ((not key->is_primitive()) || key->is<Null>())
            {
                throw Frost_Recoverable_Error{
                    fmt::format("Map keys may only be primitive values, got {}",
                                key->type_name())};
            }
        }

        value_.emplace<Map>(std::move(map));
    }

    // Constructor tag for _trusted_ map input (keys are int, bool float, or
    // string)
    struct Trusted
    {
    } constexpr static inline trusted{};

    Value(Trusted, Map&& map)
        : value_{std::move(map)}
    {
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

    static std::optional<Value_Ptr> index_array(const Array& array, Int index);

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
        std::make_shared<Value>(singleton_tag, Null{});
    static inline Value_Ptr true_singleton_ =
        std::make_shared<Value>(singleton_tag, true);
    static inline Value_Ptr false_singleton_ =
        std::make_shared<Value>(singleton_tag, false);
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
