#ifndef FROST_UTILS_HPP
#define FROST_UTILS_HPP

#include <expected>
#include <flat_set>
#include <string>
#include <string_view>
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

bool is_identifier_like(const std::string& key);

using Fmt_Segment = std::variant<Fmt_Literal, Fmt_Placeholder>;

std::expected<std::vector<Fmt_Segment>, std::string> parse_fmt_string(
    const std::string& str);

#define FROST_X_KEYWORDS                                                       \
    X("if")                                                                    \
    X("else")                                                                  \
    X("elif")                                                                  \
    X("def")                                                                   \
    X("export")                                                                \
    X("fn")                                                                    \
    X("reduce")                                                                \
    X("map")                                                                   \
    X("foreach")                                                               \
    X("filter")                                                                \
    X("with")                                                                  \
    X("init")                                                                  \
    X("true")                                                                  \
    X("false")                                                                 \
    X("and")                                                                   \
    X("or")                                                                    \
    X("not")                                                                   \
    X("null")

const inline std::flat_set<std::string_view> reserved_keywords{
#define X(keyword) keyword,
    FROST_X_KEYWORDS
#undef X
};

[[nodiscard]] inline bool is_reserved_keyword(std::string_view value)
{
    return reserved_keywords.contains(value);
}
} // namespace frst::utils

#endif
