#include <frost/value.hpp>

#include <fmt/format.h>

#include <charconv>
#include <ranges>

using namespace std::literals;

namespace frst
{

struct To_String_Impl
{
    std::string operator()(const Null&,
                           [[maybe_unused]] bool in_structure = false) const
    {
        return "null";
    }

    std::string operator()(const Int& value,
                           [[maybe_unused]] bool in_structure = false) const
    {
        char buf[24]{};
        auto [ptr, err] = std::to_chars(std::ranges::begin(buf),
                                        std::ranges::end(buf), value);
        if (err != std::errc{})
            throw Frost_Internal_Error{fmt::format(
                "Int->String error: {}", std::make_error_code(err).message())};

        return buf;
    }

    std::string operator()(const Float& value,
                           [[maybe_unused]] bool in_structure = false) const
    {
        char buf[24]{};
        auto [ptr, err] = std::to_chars(std::ranges::begin(buf),
                                        std::ranges::end(buf), value);
        if (err != std::errc{})
            throw Frost_Internal_Error{
                fmt::format("Float->String error: {}",
                            std::make_error_code(err).message())};

        return buf;
    }

    std::string operator()(const String& value, bool in_structure = false) const
    {
        if (in_structure)
            return fmt::format(R"("{}")", value);
        else
            return value;
    }

    std::string operator()(const Bool& value,
                           [[maybe_unused]] bool in_structure = false) const
    {
        return value ? "true" : "false";
    }

    std::string operator()(const Array& value,
                           [[maybe_unused]] bool in_structure = false) const
    {
        auto array_insides = value
                             | std::views::transform([&](const auto& value) {
                                   return value->to_internal_string(true);
                               })
                             | std::views::join_with(", "sv)
                             | std::ranges::to<std::string>();

        return fmt::format("[ {} ]", array_insides);
    }

    std::string operator()(const Map& value,
                           [[maybe_unused]] bool in_structure = false) const
    {
        auto map_insides =
            value
            | std::views::transform([&](const auto& kv) {
                  const auto& [k, v] = kv;
                  return fmt::format(R"([{}]: {})", k->to_internal_string(true),
                                     v->to_internal_string(true));
              })
            | std::views::join_with(", "sv)
            | std::ranges::to<std::string>();

        return fmt::format("{{ {} }}", map_insides);
    }

    std::string operator()(const Function&,
                           [[maybe_unused]] bool in_structure = false) const
    {
        return "<Function>";
    }

} constexpr static to_string_impl;

std::string Value::to_internal_string(bool in_structure) const
{
    return value_.visit([&](const auto& value) {
        return to_string_impl(value, in_structure);
    });
}

struct To_Int_Impl
{
    static std::optional<Int> operator()(const String& value)
    {
        Int result{};
        auto begin = value.data();
        auto end = begin + value.size();
        auto [ptr, err] = std::from_chars(begin, end, result);

        if (err != std::error_code{})
            return std::nullopt;

        if (ptr != end)
            return std::nullopt;

        return result;
    }

    static std::optional<Int> operator()(const auto&)
    {
        return std::nullopt;
    }
} constexpr static to_int_impl;

std::optional<Int> Value::to_internal_int() const
{
    return as<Int>().or_else([&] {
        return value_.visit(to_int_impl);
    });
}

Value_Ptr Value::to_int() const
{
    return to_internal_int()
        .transform([](const Int v) {
            return create(auto{v});
        })
        .value_or(Value::null());
}

struct To_Float_Impl
{
    static std::optional<Float> operator()(const String& value)
    {
        Float result{};
        auto begin = value.data();
        auto end = begin + value.size();
        auto [ptr, err] = std::from_chars(begin, end, result);

        if (err != std::error_code{})
            return std::nullopt;

        if (ptr != end)
            return std::nullopt;

        return result;
    }

    static std::optional<Float> operator()(const auto&)
    {
        return std::nullopt;
    }
} constexpr static to_float_impl;

std::optional<Float> Value::to_internal_float() const
{
    return as<Float>().or_else([&] {
        return value_.visit(to_float_impl);
    });
}

Value_Ptr Value::to_float() const
{
    return to_internal_float()
        .transform([](const Float v) {
            return create(auto{v});
        })
        .value_or(Value::null());
}

} // namespace frst
