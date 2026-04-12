#include <frost/ast/ast-node.hpp>
#include <frost/parser.hpp>

#include <boost/json.hpp>

#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>
#include <string_view>

namespace json = boost::json;
using frst::ast::AST_Node;

constexpr static const char html_template[]{
#embed "template.html"
};

static json::object node_to_json(const AST_Node& node, int& next_id)
{
    json::object obj;
    obj["id"] = next_id++;
    obj["label"] = node.node_label();

    const auto range = node.source_range();
    const auto no = AST_Node::no_range;
    if (range.begin.line != no.begin.line or range.begin.column != no.begin.column)
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

    // Parse
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

    json::object data;
    data["filename"] = path.filename().string();
    data["source"] = source;
    data["ast"] = std::move(ast);

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
