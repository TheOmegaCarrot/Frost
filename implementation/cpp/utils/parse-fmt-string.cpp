#include <frost/utils.hpp>

#include <fmt/format.h>

namespace frst::utils
{

namespace
{
bool is_identifier_like(const std::string& key)
{
    if (key.empty())
    {
        return false;
    }

    auto is_alpha = [](char c) {
        return (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z');
    };
    auto is_digit = [](char c) {
        return c >= '0' && c <= '9';
    };
    auto is_start = [&](char c) {
        return is_alpha(c) || c == '_';
    };
    auto is_continue = [&](char c) {
        return is_start(c) || is_digit(c);
    };

    if (!is_start(key.front()))
    {
        return false;
    }
    for (std::size_t i = 1; i < key.size(); ++i)
    {
        if (!is_continue(key.at(i)))
        {
            return false;
        }
    }

    return true;
}
} // namespace

std::expected<std::vector<Fmt_Segment>, std::string> parse_fmt_string(
    const std::string& str)
{
    std::vector<Fmt_Segment> sections;
    const auto len = str.size();
    std::size_t i = 0;
    std::string literal;

    auto flush_literal = [&]() {
        if (!literal.empty())
        {
            sections.push_back(Fmt_Literal{std::move(literal)});
            literal.clear();
        }
    };

    while (i < len)
    {
        const char c = str.at(i);

        if (c == '\\')
        {
            if (i + 1 < len)
            {
                const char next = str.at(i + 1);
                if (next == '$' || next == '\\')
                {
                    literal.push_back(next);
                    i += 2;
                    continue;
                }
            }

            literal.push_back('\\');
            ++i;
            continue;
        }

        if (c == '$' && i + 1 < len && str.at(i + 1) == '{')
        {
            const auto end = str.find('}', i + 2);
            if (end == std::string::npos)
            {
                return std::unexpected{fmt::format(
                    "Unterminated format placeholder: {}", str.substr(i))};
            }

            const auto content = str.substr(i + 2, end - (i + 2));
            if (!is_identifier_like(content))
            {
                if (content.empty())
                {
                    return std::unexpected{"Invalid format placeholder: ${}"};
                }
                return std::unexpected{fmt::format(
                    "Invalid format placeholder: ${{{}}}", content)};
            }

            flush_literal();
            sections.push_back(Fmt_Placeholder{content});
            i = end + 1;
            continue;
        }

        literal.push_back(c);
        ++i;
    }

    flush_literal();
    return sections;
}

} // namespace frst::utils
