// frost-highlight -- syntax highlighter for Frost source code
// Reads Frost source from stdin, writes highlighted HTML to stdout.
// Uses tree-sitter with the Frost grammar and highlight queries.

#include <tree_sitter/api.h>

#include <algorithm>
#include <iostream>
#include <iterator>
#include <string>
#include <vector>

extern "C"
{
    const TSLanguage* tree_sitter_frost(void);
}

constexpr static const char highlights_scm[]{
#embed "highlights.scm"
};

struct Highlight
{
    uint32_t start;
    uint32_t end;
    std::string scope;
};

static std::vector<Highlight> get_highlights(const std::string& source)
{
    auto* parser = ts_parser_new();
    ts_parser_set_language(parser, tree_sitter_frost());

    auto* tree = ts_parser_parse_string(parser, nullptr, source.c_str(),
                                        static_cast<uint32_t>(source.size()));

    uint32_t error_offset;
    TSQueryError error_type;
    auto* query =
        ts_query_new(tree_sitter_frost(), highlights_scm,
                     sizeof(highlights_scm), &error_offset, &error_type);

    if (not query)
    {
        ts_tree_delete(tree);
        ts_parser_delete(parser);
        return {};
    }

    auto* cursor = ts_query_cursor_new();
    ts_query_cursor_exec(cursor, query, ts_tree_root_node(tree));

    std::vector<Highlight> highlights;
    TSQueryMatch match;
    uint32_t capture_index;
    while (ts_query_cursor_next_capture(cursor, &match, &capture_index))
    {
        const auto& capture = match.captures[capture_index];

        uint32_t name_len;
        const auto* name =
            ts_query_capture_name_for_id(query, capture.index, &name_len);

        auto scope = std::string(name, name_len);
        std::ranges::replace(scope, '.', '-');

        highlights.push_back({
            ts_node_start_byte(capture.node),
            ts_node_end_byte(capture.node),
            std::move(scope),
        });
    }

    ts_query_cursor_delete(cursor);
    ts_query_delete(query);
    ts_tree_delete(tree);
    ts_parser_delete(parser);

    return highlights;
}

static void write_escaped(std::ostream& out, char c)
{
    switch (c)
    {
    case '&': out << "&amp;"; break;
    case '<': out << "&lt;"; break;
    case '>': out << "&gt;"; break;
    default: out << c; break;
    }
}

int main()
{
    auto source = std::string{
        std::istreambuf_iterator<char>{std::cin}, {}};

    auto highlights = get_highlights(source);

    // Build a per-byte scope map. Last capture wins (tree-sitter
    // highlight convention: more specific captures come later).
    std::vector<int> byte_scope(source.size(), -1);
    for (int i = 0; i < static_cast<int>(highlights.size()); ++i)
    {
        const auto& hl = highlights[static_cast<std::size_t>(i)];
        for (auto b = hl.start; b < hl.end && b < source.size(); ++b)
            byte_scope[b] = i;
    }

    // Emit HTML spans, merging consecutive bytes with the same scope.
    int current_scope = -1;
    for (std::size_t i = 0; i < source.size(); ++i)
    {
        int scope = byte_scope[i];
        if (scope != current_scope)
        {
            if (current_scope != -1)
                std::cout << "</span>";
            if (scope != -1)
                std::cout << "<span class=\"hl-"
                          << highlights[static_cast<std::size_t>(scope)].scope
                          << "\">";
            current_scope = scope;
        }
        write_escaped(std::cout, source[i]);
    }

    if (current_scope != -1)
        std::cout << "</span>";

    return 0;
}
