// astviz -- interactive AST visualizer for Frost
// Written by Claude (Anthropic), with design direction from Ethan Hancock

#include <frost/ast/ast-node.hpp>
#include <frost/parser.hpp>

#include <boost/json.hpp>
#include <tree_sitter/api.h>

#include <algorithm>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>
#include <string_view>

extern "C"
{
    const TSLanguage* tree_sitter_frost(void);
}

namespace json = boost::json;
using frst::ast::AST_Node;

constexpr static const char html_template[]{
#embed "template.html"
};

constexpr static const char highlights_scm[]{
#embed "highlights.scm"
};

// --- AST serialization ---

static json::object node_to_json(const AST_Node& node, int& next_id)
{
    json::object obj;
    obj["id"] = next_id++;
    obj["label"] = node.node_label();

    const auto range = node.source_range();
    const auto no = AST_Node::no_range;
    if (range.begin.line != no.begin.line
        or range.begin.column != no.begin.column)
    {
        obj["range"] = json::object{
            {"sl", static_cast<std::int64_t>(range.begin.line)},
            {"sc", static_cast<std::int64_t>(range.begin.column)},
            {"el", static_cast<std::int64_t>(range.end.line)},
            {"ec", static_cast<std::int64_t>(range.end.column)},
        };
    }

    json::array children;
    for (const auto& child : node.children())
    {
        if (child.label.empty())
        {
            children.push_back(node_to_json(*child.node, next_id));
        }
        else
        {
            json::object wrapper;
            wrapper["id"] = next_id++;
            wrapper["label"] = child.label;
            wrapper["wrapper"] = true;
            wrapper["children"] = json::array{
                node_to_json(*child.node, next_id)};
            children.push_back(std::move(wrapper));
        }
    }

    if (not children.empty())
        obj["children"] = std::move(children);

    return obj;
}

// --- Tree-sitter syntax highlighting ---

static json::array get_highlights(const std::string& source)
{
    auto* parser = ts_parser_new();
    ts_parser_set_language(parser, tree_sitter_frost());

    auto* tree = ts_parser_parse_string(
        parser, nullptr, source.c_str(),
        static_cast<uint32_t>(source.size()));

    uint32_t error_offset;
    TSQueryError error_type;
    auto* query = ts_query_new(
        tree_sitter_frost(), highlights_scm, sizeof(highlights_scm),
        &error_offset, &error_type);

    if (not query)
    {
        std::cerr << "Warning: highlight query failed at offset "
                  << error_offset << "\n";
        ts_tree_delete(tree);
        ts_parser_delete(parser);
        return {};
    }

    auto* cursor = ts_query_cursor_new();
    ts_query_cursor_exec(cursor, query, ts_tree_root_node(tree));

    json::array highlights;
    TSQueryMatch match;
    uint32_t capture_index;
    while (ts_query_cursor_next_capture(cursor, &match, &capture_index))
    {
        const auto& capture = match.captures[capture_index];

        uint32_t name_len;
        const auto* name = ts_query_capture_name_for_id(
            query, capture.index, &name_len);

        auto scope = std::string(name, name_len);
        std::ranges::replace(scope, '.', '-');

        json::object hl;
        hl["s"] = static_cast<std::int64_t>(ts_node_start_byte(capture.node));
        hl["e"] = static_cast<std::int64_t>(ts_node_end_byte(capture.node));
        hl["c"] = std::move(scope);
        highlights.push_back(std::move(hl));
    }

    ts_query_cursor_delete(cursor);
    ts_query_delete(query);
    ts_tree_delete(tree);
    ts_parser_delete(parser);

    return highlights;
}

// --- Main ---

int main(int argc, char** argv)
{
    if (argc != 2)
    {
        std::cerr << "Usage: astviz <file.frst>\n";
        return 1;
    }

    const auto path = std::filesystem::path{argv[1]};

    // Read source text for embedding in HTML
    std::ifstream ifs{path};
    if (not ifs)
    {
        std::cerr << "Error: could not read file: " << path << "\n";
        return 1;
    }
    const auto source = std::string{
        std::istreambuf_iterator<char>{ifs}, {}};
    ifs.close();

    // Parse with Frost parser (for AST)
    auto result = frst::parse_file(path);
    if (not result)
    {
        std::cerr << result.error() << "\n";
        return 1;
    }

    // Serialize AST to JSON
    int next_id = 0;
    json::array ast;
    for (const auto& stmt : result.value())
        ast.push_back(node_to_json(*stmt, next_id));

    // Syntax highlighting via tree-sitter
    auto highlights = get_highlights(source);

    json::object data;
    data["filename"] = path.filename().string();
    data["source"] = source;
    data["ast"] = std::move(ast);
    data["highlights"] = std::move(highlights);

    const auto template_view =
        std::string_view{html_template, sizeof(html_template)};

    std::cout
        << "<!DOCTYPE html>\n"
           "<html lang=\"en\">\n"
           "<head>\n"
           "  <meta charset=\"utf-8\">\n"
           "  <title>AST: "
        << path.filename().string()
        << "</title>\n"
           "</head>\n"
           "<body>\n"
           "<script>\nwindow.FROST_DATA = "
        << json::serialize(data)
        << ";\n</script>\n"
        << template_view
        << "\n</body>\n</html>\n";

    return 0;
}
