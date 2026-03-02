#ifndef FROST_CATCH_STRINGMAKER_SPECIALIZATIONS_HPP
#define FROST_CATCH_STRINGMAKER_SPECIALIZATIONS_HPP

#include <catch2/catch_tostring.hpp>
#include <fmt/std.h>

#include <optional>

namespace Catch
{
template <typename T>
struct StringMaker<std::optional<T>>
{
    static std::string convert(const std::optional<T>& value)
    {
        return fmt::format("{}", value);
    }
};
} // namespace Catch

#endif
