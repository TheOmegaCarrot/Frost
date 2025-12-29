#include "statement.hpp"

using frst::ast::Statement;

// ChatGPT wrote this AST-dump code because I couldn't be bothered
// I just adapted it a little
void Statement::debug_dump_ast(std::ostream& out) const {
    debug_dump_ast_impl(out, "", true, true);
}

namespace {
void print_node(std::ostream& out, std::string_view prefix, bool is_last,
                std::string_view label, bool is_root) {
    if (is_root)
        out << label << "\n";
    else
        out << prefix << (is_last ? "└── " : "├── ") << label << "\n";
}

std::string child_prefix(std::string_view prefix, bool is_last, bool is_root) {
    std::string out{prefix};
    // For non-root nodes, extend the prefix: keep a vertical bar if more
    // siblings follow, otherwise add whitespace.
    if (!is_root) out += (is_last ? "    " : "│   ");
    return out;
}
}  // namespace

void Statement::debug_dump_ast_impl(std::ostream& out, std::string_view prefix,
                                    bool is_last, bool is_root) const {
    const auto label = node_label();
    print_node(out, prefix, is_last, label, is_root);

    // Children are exposed as a 0-based sequence. child_at() returns
    // {nullptr, {}} when there are no more children.
    auto child = child_at(0);
    if (!child) return;

    const auto child_prefix_ = child_prefix(prefix, is_last, is_root);
    for (std::size_t i = 0; child->node; ++i) {
        // Pre-fetch the next child to decide whether the current one is
        // the last sibling.
        const auto next_child = child_at(i + 1u);
        const bool child_is_last = next_child.has_value();
        if (child->label.empty()) {
            child->node->debug_dump_ast_impl(out, child_prefix_, child_is_last,
                                             false);
        } else {
            // Labeled children get an extra label node, then the real child
            // is printed as the only descendant of that label.
            print_node(out, child_prefix_, child_is_last, child->label, false);
            const auto labeled_prefix =
                child_prefix(child_prefix_, child_is_last, false);
            child->node->debug_dump_ast_impl(out, labeled_prefix, true, false);
        }

        child = next_child;
    }
}
