#include <frost/value.hpp>

#include <fmt/format.h>

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
        return std::to_string(value);
    }

    std::string operator()(const Float& value,
                           [[maybe_unused]] bool in_structure = false) const
    {
        return std::to_string(value);
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
        auto array_insides =
            value | std::views::transform([&](const auto& value) {
                return value->to_internal_string(true);
            }) |
            std::views::join_with(", "sv) | std::ranges::to<std::string>();

        return fmt::format("[ {} ]", array_insides);
    }

    std::string operator()(const Map& value,
                           [[maybe_unused]] bool in_structure = false) const
    {
        auto map_insides =
            value | std::views::transform([&](const auto& kv) {
                const auto& [k, v] = kv;
                return fmt::format(R"([{}]: {})", k->to_internal_string(true),
                                   v->to_internal_string(true));
            }) |
            std::views::join_with(", "sv) | std::ranges::to<std::string>();

        return fmt::format("{{ {} }}", map_insides);
    }

} constexpr static to_string_impl;

std::string Value::to_internal_string(bool in_structure) const
{
    return value_.visit(
        [&](const auto& value) { return to_string_impl(value, in_structure); });
}

} // namespace frst
