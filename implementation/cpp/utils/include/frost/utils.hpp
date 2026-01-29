#ifndef FROST_UTILS_HPP
#define FROST_UTILS_HPP

#include <cstddef>
#include <expected>
#include <string>
#include <vector>

namespace frst::utils
{
struct Replacement_Section
{
    std::size_t start;
    std::size_t len;
    std::string content;
};

std::expected<std::vector<Replacement_Section>, std::string> parse_fmt_string(
    const std::string& str);
} // namespace frst::utils

#endif
