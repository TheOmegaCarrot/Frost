#ifndef FROST_BUILTIN_HPP
#define FROST_BUILTIN_HPP

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <fmt/format.h>

#include <functional>
#include <optional>
#include <span>

namespace frst
{
using builtin_args_t = std::span<const Value_Ptr>;

// Class wrapping a C++ function exposed to Frost
// e.g. builtins
class Builtin final : public Callable
{
  private:
    using function_t = std::function<Value_Ptr(builtin_args_t)>;

  public:
    using Ptr = std::shared_ptr<Builtin>;

    struct Arity
    {
        std::size_t min;
        std::optional<std::size_t> max;
    };

    Builtin() = delete;
    Builtin(const Builtin&) = delete;
    Builtin(Builtin&&) = delete;
    Builtin& operator=(const Builtin&) = delete;
    Builtin& operator=(Builtin&&) = delete;
    ~Builtin() final = default;

    // nullopt max_arity indicates a variadic builtin function
    Builtin(function_t function, std::string name, const Arity& arity);

    Value_Ptr call(builtin_args_t args) const final;

    std::string debug_dump() const final;

  private:
    function_t function_;
    std::string name_;
    Arity arity_;
};

#ifdef FROST_ENABLE_HTTP
#define OPT_HTTP X(http)
#else
#define OPT_HTTP
#endif

#define X_INJECT                                                               \
    X(structure_ops)                                                           \
    X(type_checks)                                                             \
    X(type_conversions)                                                        \
    X(pack_call)                                                               \
    X(debug_helpers)                                                           \
    X(error_handling)                                                          \
    X(output)                                                                  \
    X(free_operators)                                                          \
    X(math)                                                                    \
    X(string_ops)                                                              \
    X(regex)                                                                   \
    X(mutable_cell)                                                            \
    X(ranges)                                                                  \
    X(and_then)                                                                \
    X(os)                                                                      \
    X(streams)                                                                 \
    X(json)                                                                    \
    X(filesystem)                                                              \
    OPT_HTTP

#define X(F) void inject_##F(Symbol_Table&);

X_INJECT

#undef X

inline void inject_builtins(Symbol_Table& table)
{
#define X(F) inject_##F(table);

    X_INJECT

#undef X
}

} // namespace frst

#endif
