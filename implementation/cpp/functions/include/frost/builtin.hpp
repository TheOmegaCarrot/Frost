#ifndef FROST_BUILTIN_HPP
#define FROST_BUILTIN_HPP

#include <frost/value.hpp>

#include <fmt/format.h>

#include <functional>
#include <optional>

namespace frst
{
using builtin_args_t = const std::vector<Value_Ptr>&;

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
    Builtin(function_t function, std::string name, const Arity& arity)
        : function_{std::move(function)}
        , name_{std::move(name)}
        , arity_{arity}
    {
    }

    Value_Ptr call(builtin_args_t args) const final
    {
        if (auto num_args = args.size(); arity_.max && num_args > arity_.max)
        {
            throw Frost_Error{
                fmt::format("Function {} called with too many arguments. "
                            "Called with {} but accepts no more than {}.",
                            name_, num_args, arity_.max.value())};
        }
        else if (num_args < arity_.min)
        {
            throw Frost_Error{
                fmt::format("Function {} called with insufficient arguments. "
                            "Called with {} but requires at least {}.",
                            name_, num_args, arity_.min)};
        }

        return function_(args);
    }

    std::string debug_dump() const final
    {
        return fmt::format("<builtin:{}>", name_);
    }

  private:
    function_t function_;
    std::string name_;
    Arity arity_;
};

void inject_builtins(Symbol_Table& table);

} // namespace frst

#endif
