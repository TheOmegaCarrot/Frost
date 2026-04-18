#ifndef FROST_BUILTIN_HPP
#define FROST_BUILTIN_HPP

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <fmt/format.h>

#include <functional>
#include <span>

namespace frst
{
using builtin_args_t = std::span<const Value_Ptr>;

// Class wrapping a C++ function exposed to Frost
// e.g. builtins
class Builtin : public Callable
{
  private:
    using function_t = std::function<Value_Ptr(builtin_args_t)>;

  public:
    using Ptr = std::shared_ptr<Builtin>;

    Builtin() = delete;
    Builtin(const Builtin&) = delete;
    Builtin(Builtin&&) = delete;
    Builtin& operator=(const Builtin&) = delete;
    Builtin& operator=(Builtin&&) = delete;
    ~Builtin() override = default;

    Builtin(function_t function, std::string name);

    Value_Ptr call(builtin_args_t args) const override;

    std::string debug_dump() const final;
    std::string name() const final;

  private:
    function_t function_;
    std::string name_;
};

void inject_builtins(Symbol_Table& table);

} // namespace frst

#endif
