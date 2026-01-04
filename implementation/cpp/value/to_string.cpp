#include <frost/value.hpp>

#include <fmt/format.h>

#include <ranges>

namespace frst
{

struct To_String_Impl
{
    std::string operator()(const Null&,
                           [[maybe_unused]] bool in_structure = false)
    {
        return "null";
    }

    std::string operator()(const Int& value,
                           [[maybe_unused]] bool in_structure = false)
    {
        return std::to_string(value);
    }

    std::string operator()(const Float& value,
                           [[maybe_unused]] bool in_structure = false)
    {
        return std::to_string(value);
    }

    std::string operator()(const String& value, bool in_structure = false)
    {
        if (in_structure)
            return fmt::format(R"("{}")", value);
        else
            return value;
    }

    std::string operator()(const Bool& value,
                           [[maybe_unused]] bool in_structure = false)
    {
        return std::to_string(value);
    }

    std::string operator()(this auto& self, const Array& value,
                           [[maybe_unused]] bool in_structure = false)
    {
        auto array_insides = value |
                             std::views::transform([&](const auto& value) {
                                 return self(value, true);
                             }) |
                             std::views::join_with(", ");

        return fmt::format("[ {} ]", array_insides);
    }

    std::string operator()(this auto& self, const Map& value,
                           [[maybe_unused]] bool in_structure = false)
    {
        auto map_insides =
            value | std::views::transform([&](const auto& kv) {
                const auto& [k, v] = kv;
                return fmt::format(R"([{}]: {})", self(k, true), self(v, true));
            }) |
            std::views::join_with(", ");

        return fmt::format("{{ {} }}", map_insides);
    }

} constexpr static to_string_impl;

std::string Value::to_internal_string() const
{
    return value_.visit(to_string_impl);
}

} // namespace frst
