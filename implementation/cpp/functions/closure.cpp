#include <frost/ast.hpp>
#include <frost/closure.hpp>
#include <frost/symbol-table.hpp>

#include <set>

using namespace frst;

namespace
{
std::generator<ast::Statement::Symbol_Action> node_to_sym_seq(
    const ast::Statement::Ptr& node)
{
    return node->symbol_sequence();
}
} // namespace

Closure::Closure(std::vector<std::string> parameters,
                 std::vector<ast::Statement::Ptr> body,
                 const Symbol_Table& construction_environment)
    : parameters_{std::move(parameters)}
    , body_{std::move(body)}
{
    std::set<std::string> names_defined_so_far{std::from_range, parameters_};
    std::set<std::string> names_to_capture;

    for (const ast::Statement::Symbol_Action& name :
         body_ | std::views::transform(&node_to_sym_seq) | std::views::join)
    {
        name.visit(Overload{
            [&](const ast::Statement::Definition& defn) {
                names_defined_so_far.insert(defn.name);
            },
            [&](const ast::Statement::Usage& used) {
                if (!names_defined_so_far.contains(used.name))
                    names_to_capture.insert(used.name);
            },
        });
    }

    for (const std::string& name : names_to_capture)
    {
        if (not construction_environment.has(name))
        {
            throw Frost_Error{fmt::format(
                "No definition found for captured symbol: {}", name)};
        }

        captures_.define(name, construction_environment.lookup(name));
    }
}

Value_Ptr eval_or_null(const ast::Statement::Ptr& node,
                       const Symbol_Table& syms)
{
    if (auto expr_ptr = dynamic_cast<ast::Expression*>(node.get()))
        return expr_ptr->evaluate(syms);
    return Value::create(Null{});
}

Value_Ptr Closure::call(const std::vector<Value_Ptr>& args) const
{
    if (body_.size() == 0)
        return Value::create(Null{});

    Symbol_Table exec_table(&captures_);

    for (const ast::Statement::Ptr& node : body_
                                               | std::views::reverse
                                               | std::views::drop(1)
                                               | std::views::reverse)
    {
        node->execute(exec_table);
    }

    return eval_or_null(body_.back(), exec_table);
}

std::string Closure::debug_dump() const
{
    std::ostringstream os;
    os << "<Closure>";

    if (not captures_.debug_table().empty())
    {
        os
            << " (capturing: "
            << (captures_.debug_table()
                | std::views::keys
                | std::views::join_with(',')
                | std::ranges::to<std::string>())
            << ")\n";
    }

    for (const auto& statement : body_)
    {
        os << "    ";
        statement->debug_dump_ast(os);
    }

    return std::move(os).str();
}
