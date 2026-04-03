#include <frost/utils.hpp>

namespace frst::utils
{

std::expected<std::string, std::string> trim_multiline_indentation(
    std::string_view content)
{
    // Single-line content (no newlines) passes through unchanged.
    if (content.find('\n') == std::string_view::npos)
        return std::string{content};

    // Split into lines.
    std::vector<std::string_view> lines;
    std::size_t pos = 0;
    while (pos <= content.size())
    {
        auto nl = content.find('\n', pos);
        if (nl == std::string_view::npos)
        {
            lines.push_back(content.substr(pos));
            break;
        }
        lines.push_back(content.substr(pos, nl - pos));
        pos = nl + 1;
    }

    // The last line defines the indentation prefix.
    // If it contains non-whitespace, that's an error -- the closing
    // delimiter must be the first syntax on its own line.
    auto last = lines.back();
    for (char c : last)
    {
        if (c != ' ' && c != '\t')
        {
            return std::unexpected{
                "closing delimiter must be on its own line, or "
                "content appears to the left of the closing delimiter"};
        }
    }

    auto prefix = last;

    // Remove the last line (it's the delimiter's indentation, not content).
    lines.pop_back();

    // Remove the first line if it's empty (the opening delimiter was
    // immediately followed by a newline).
    if (not lines.empty() && lines.front().empty())
        lines.erase(lines.begin());

    // Strip the prefix from each line.
    for (auto& line : lines)
    {
        // Empty lines are preserved as empty.
        if (line.empty())
            continue;

        if (not line.starts_with(prefix))
        {
            return std::unexpected{
                "line is indented less than the closing delimiter"};
        }
        line.remove_prefix(prefix.size());
    }

    // Rejoin.
    std::string result;
    for (std::size_t i = 0; i < lines.size(); ++i)
    {
        if (i > 0)
            result += '\n';
        result += lines[i];
    }

    return result;
}

} // namespace frst::utils
