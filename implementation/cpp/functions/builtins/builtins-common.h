#ifndef FROST_BUILTINS_COMMON_HPP
#define FROST_BUILTINS_COMMON_HPP

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>

#include <array>
#include <cstddef>
#include <string_view>

#define INJECT(NAME, MIN_ARITY, MAX_ARITY)                                     \
    table.define(#NAME,                                                        \
                 Value::create(Function{std::make_shared<Builtin>(             \
                     &NAME, #NAME,                                             \
                     Builtin::Arity{.min = MIN_ARITY, .max = MAX_ARITY})}))

#define INJECT_V(NAME, MIN_ARITY) INJECT(NAME, MIN_ARITY, std::nullopt)

// Below is a GPT-generated boilerplate generation system for type-checking
// arguments to builtins

namespace frst::builtin_detail
{
template <typename... Ts>
struct Types
{
};

template <typename... Ts>
struct ParamSpec
{
    Types<Ts...> types;
    std::string_view label;
};

template <typename... Ts>
constexpr bool matches(const Value_Ptr& v)
{
    return (v->is<Ts>() || ...);
}

template <typename... Ts>
std::string expected_list()
{
    std::array<std::string_view, sizeof...(Ts)> names{type_str<Ts>()...};
    std::string out;
    for (std::size_t i = 0; i < names.size(); ++i)
    {
        if (i > 0)
            out += " or ";
        out += names[i];
    }
    return out;
}

template <typename... Ts>
void require_arg(std::string_view fn, builtin_args_t args, std::size_t idx,
                 Types<Ts...>, std::string_view label = {})
{
    const auto& arg = args.at(idx);
    if (!matches<Ts...>(arg))
    {
        const auto label_part =
            label.empty() ? fmt::format("argument {}", idx + 1)
                          : fmt::format("argument {} ({})", idx + 1, label);
        throw Frost_Error{fmt::format("Function {} requires {} as {}, got {}",
                                      fn, expected_list<Ts...>(), label_part,
                                      arg->type_name())};
    }
}

template <typename... Ts>
void require_arg(std::string_view fn, builtin_args_t args, std::size_t idx,
                 ParamSpec<Ts...> spec)
{
    require_arg(fn, args, idx, spec.types, spec.label);
}

template <typename... Specs>
void require_args(std::string_view fn, builtin_args_t args, Specs... specs)
{
    std::size_t i = 0;
    (require_arg(fn, args, i++, specs), ...);
}
} // namespace frst::builtin_detail

#define TYPES(...)                                                             \
    frst::builtin_detail::Types<__VA_ARGS__>                                   \
    {                                                                          \
    }

#define PARAM(LABEL, TYPES_SPEC)                                               \
    frst::builtin_detail::ParamSpec                                            \
    {                                                                          \
        TYPES_SPEC, LABEL                                                      \
    }

#define REQUIRE_ARGS(NAME, ...)                                                \
    frst::builtin_detail::require_args(#NAME, args __VA_OPT__(, ) __VA_ARGS__)

#endif
