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

std::expected<std::vector<Replacement_Section>, std::string> parse_fmt_string(
    const std::string& str)
{
    std::vector<Replacement_Section> sections;
    const auto len = str.size();
    std::size_t i = 0;

    while (i < len)
    {
        if (str.at(i) != '$')
        {
            ++i;
            continue;
        }

        if (i > 0)
        {
            std::size_t backslashes = 0;
            for (std::size_t j = i; j > 0 && str.at(j - 1) == '\\'; --j)
            {
                ++backslashes;
            }
            if ((backslashes % 2) == 1)
            {
                ++i;
                continue;
            }
        }

        if (i + 1 >= len)
        {
            ++i;
            continue;
        }

        if (str.at(i + 1) != '{')
        {
            ++i;
            continue;
        }

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
            return std::unexpected{
                fmt::format("Invalid format placeholder: ${{{}}}", content)};
        }

        sections.push_back(Replacement_Section{
            .start = i,
            .len = end - i + 1,
            .content = content,
        });
        i = end + 1;
    }

    return sections;
}

} // namespace frst::utils
