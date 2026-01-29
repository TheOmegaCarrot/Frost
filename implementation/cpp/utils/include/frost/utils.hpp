#ifndef FROST_UTILS_HPP
#define FROST_UTILS_HPP

#include <cstddef>
#include <expected>
#include <string>
#include <variant>
#include <vector>

namespace frst::utils
{
struct Fmt_Literal
{
    std::string text;
};

struct Fmt_Placeholder
{
    std::string text;
};

using Fmt_Segment = std::variant<Fmt_Literal, Fmt_Placeholder>;

std::expected<std::vector<Fmt_Segment>, std::string> parse_fmt_string(
    const std::string& str);
} // namespace frst::utils

#endif
