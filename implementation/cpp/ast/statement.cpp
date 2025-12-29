#include "statement.hpp"

using frst::ast::Statement;

// ChatGPT wrote this AST-dump code because I couldn't be bothered
// I just adapted it a little
void Statement::debug_dump_ast(std::ostream& out) const
{
    debug_dump_ast_impl(
        out, Print_Context{.prefix = "", .is_last = true, .is_root = true});
}

void Statement::debug_dump_ast_impl(std::ostream& out,
                                    const Print_Context& context) const
{
    const auto label = node_label();
    print_node(out, context, label);

    const auto child_prefix_ = child_prefix(context);
    Child_Info previous{};
    bool has_previous = false;
    for (auto child : children())
    {
        if (has_previous)
            print_child(out, Print_Context{child_prefix_, false, false},
                        previous);
        previous = child;
        has_previous = true;
    }

    if (has_previous)
        print_child(out, Print_Context{child_prefix_, true, false}, previous);
}

void Statement::print_node(std::ostream& out, const Print_Context& context,
                           std::string_view label)
{
    if (context.is_root)
        out << label << "\n";
    else
        out << context.prefix << (context.is_last ? "└── " : "├── ") << label
            << "\n";
}

void Statement::print_child(std::ostream& out, const Print_Context& context,
                            const Child_Info& child)
{

    if (child.label.empty())
    {
        child.node->debug_dump_ast_impl(
            out, Print_Context{context.prefix, context.is_last, false});
        return;
    }

    // Labeled children get an extra label node, then the real child
    // is printed as the only descendant of that label.
    print_node(out, context, child.label);
    const auto labeled_prefix = child_prefix(context);
    child.node->debug_dump_ast_impl(out,
                                    Print_Context{labeled_prefix, true, false});
}
