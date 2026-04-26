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

std::expected<std::string, std::string> trim_multiline_indentation(
    std::string_view content);

#define FROST_X_KEYWORDS                                                       \
    X("if")                                                                    \
    X("else")                                                                  \
    X("elif")                                                                  \
    X("def")                                                                   \
    X("defn")                                                                  \
    X("do")                                                                    \
    X("export")                                                                \
    X("fn")                                                                    \
    X("reduce")                                                                \
    X("map")                                                                   \
    X("foreach")                                                               \
    X("filter")                                                                \
    X("with")                                                                  \
    X("init")                                                                  \
    X("match")                                                                 \
    X("as")                                                                    \
    X("is")                                                                    \
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

// `$`, `$1`-`$9`, `$$` -- placeholder parameter names for abbreviated lambdas.
// These may only appear as name lookups inside `$(...)`, never as definition
// targets.
[[nodiscard]] inline bool is_dollar_identifier(std::string_view value)
{
    if (not value.starts_with('$'))
        return false;
    // "$"
    if (value.size() == 1)
        return true;
    // "$$"
    if (value == "$$")
        return true;
    // "$N" (single digit)
    return (value.size() == 2) && (value[1] >= '0') && (value[1] <= '9');
}

} // namespace frst::utils

#endif
