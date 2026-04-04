#ifndef FROST_BUILTINS_COMMON_HPP
#define FROST_BUILTINS_COMMON_HPP

#include <frost/builtin.hpp>
#include <frost/stdlib.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <boost/preprocessor.hpp>

#include <array>
#include <cstddef>
#include <ranges>
#include <string_view>

#define BUILTIN(NAME) Value_Ptr NAME([[maybe_unused]] builtin_args_t args)

#define MAKE_BUILTIN(NAME)                                                     \
    Value::create(Function{std::make_shared<Builtin>(NAME, #NAME)})

#define MAKE_NS_BUILTIN(NS, NAME)                                              \
    Value::create(Function{std::make_shared<Builtin>(NS::NAME, #NAME)})

#define ENTRY(NAME) {Value::create(String{#NAME}), MAKE_BUILTIN(NAME)}

#define NS_ENTRY(NS, NAME)                                                     \
    {Value::create(String{#NAME}), MAKE_NS_BUILTIN(NS, NAME)}

#define INJECT(NAME) table.define(#NAME, MAKE_BUILTIN(NAME))

#define INJECT_MAP(NAME, ...)                                                  \
    table.define(#NAME, Value::create(Value::trusted, Map{__VA_ARGS__}));

#define REGISTRY_MODULE(toplevel, name, ...)                                   \
    void register_module_##name(Stdlib_Registry_Builder& builder)              \
    {                                                                          \
        using namespace name;                                                  \
        builder.register_module(                                               \
            Stdlib_Registry_Builder::module_path_t({#toplevel, #name}),        \
            Value::create(Value::trusted, Map{__VA_ARGS__}));                  \
    }

#define STDLIB_MODULE(name, ...) REGISTRY_MODULE(std, name, __VA_ARGS__)

#define UNIFORM_VARIADIC(NAME, TYPE)                                           \
    for (const auto& [i, arg] : std::views::enumerate(args))                   \
        if (not arg->is<TYPE>())                                               \
            throw Frost_Recoverable_Error{fmt::format(                         \
                "Function " #NAME " requires " #TYPE " for all arguments, "    \
                "got {} for argument {}",                                      \
                arg->type_name(), i)};

#define COERCE(IDX, TYPE)                                                      \
    (args.at(IDX)                                                              \
         ->as<TYPE>()                                                          \
         .or_else([] -> std::optional<TYPE> {                                  \
             THROW_UNREACHABLE;                                                \
         })                                                                    \
         .value())

#define GET(IDX, TYPE) (args.at(IDX)->raw_get<TYPE>())

#define HAS(IDX) (IDX < args.size())

#define IS(IDX, TYPE) (args.at(IDX)->is<TYPE>())

#define ONE_STRING(r, d, S)                                                    \
    Value_Ptr S = BOOST_PP_CAT(BOOST_PP_STRINGIZE(S), _s);
#define STRINGS(...)                                                           \
    struct                                                                     \
    {                                                                          \
        BOOST_PP_SEQ_FOR_EACH(ONE_STRING, _,                                   \
                              BOOST_PP_VARIADIC_TO_SEQ(__VA_ARGS__))           \
    } const static strings

namespace frst
{
template <typename F>
auto system_function(F&& fn)
{
    return Function{
        std::make_shared<Builtin>(std::forward<F>(fn), "<system closure>")};
}
template <typename F>
auto system_closure(F&& fn)
{
    return Value::create(system_function(std::forward<F>(fn)));
}
} // namespace frst

namespace frst::builtin_detail
{
struct Any
{
};

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

template <typename Spec>
struct Optional
{
    Spec spec;
};

template <typename T>
inline constexpr bool is_any_v = std::same_as<T, Any>;

template <typename T>
inline constexpr bool is_optional_spec_v = false;

template <typename S>
inline constexpr bool is_optional_spec_v<Optional<S>> = true;

template <typename... Ts>
constexpr bool matches(const Value_Ptr& v)
{
    if constexpr ((is_any_v<Ts> || ...))
        return true;
    else
        return (v->is<Ts>() || ...);
}

template <typename... Ts>
std::string expected_list()
{
    if constexpr ((is_any_v<Ts> || ...))
    {
        return "Any";
    }
    else
    {
        std::array<std::string_view, sizeof...(Ts)> names{type_str<Ts>()...};

        using namespace std::literals;
        return names
               | std::views::join_with(" or "sv)
               | std::ranges::to<std::string>();
    }
}

inline void require_arity(std::string_view fn, builtin_args_t args,
                          std::size_t min,
                          std::optional<std::size_t> max = std::nullopt)
{
    if (args.size() < min)
    {
        throw Frost_Recoverable_Error{
            fmt::format("Function {} called with insufficient arguments. "
                        "Called with {} but requires at least {}.",
                        fn, args.size(), min)};
    }
    if (max && args.size() > *max)
    {
        throw Frost_Recoverable_Error{
            fmt::format("Function {} called with too many arguments. "
                        "Called with {} but accepts no more than {}.",
                        fn, args.size(), *max)};
    }
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
        throw Frost_Recoverable_Error{
            fmt::format("Function {} requires {} as {}, got {}", fn,
                        expected_list<Ts...>(), label_part, arg->type_name())};
    }
}

template <typename... Ts>
void require_arg(std::string_view fn, builtin_args_t args, std::size_t idx,
                 ParamSpec<Ts...> spec)
{
    require_arg(fn, args, idx, spec.types, spec.label);
}

template <typename Spec>
void require_arg(std::string_view fn, builtin_args_t args, std::size_t idx,
                 Optional<Spec> spec)
{
    if (idx >= args.size())
        return;
    require_arg(fn, args, idx, spec.spec);
}

template <typename... Specs>
void require_args(std::string_view fn, builtin_args_t args, Specs... specs)
{
    constexpr std::size_t max_args = sizeof...(Specs);
    constexpr std::size_t min_args =
        (... + (is_optional_spec_v<Specs> ? 0 : 1));
    require_arity(fn, args, min_args, max_args);

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

#define OPTIONAL(SPEC)                                                         \
    frst::builtin_detail::Optional                                             \
    {                                                                          \
        SPEC                                                                   \
    }

#define ANY                                                                    \
    frst::builtin_detail::Types<frst::builtin_detail::Any>                     \
    {                                                                          \
    }

#define REQUIRE_ARGS(NAME, ...)                                                \
    frst::builtin_detail::require_args(NAME, args __VA_OPT__(, ) __VA_ARGS__)

#define REQUIRE_ARITY(NAME, ...)                                               \
    frst::builtin_detail::require_arity(NAME, args, __VA_ARGS__)

#define REQUIRE_NULLARY(NAME)                                                  \
    frst::builtin_detail::require_arity(NAME, args, 0, 0)

#endif
