#include <frost/ast.hpp>
#include <frost/closure.hpp>
#include <frost/symbol-table.hpp>

#include <flat_set>
#include <sstream>

using namespace frst;

Closure::Closure(std::vector<std::string> parameters,
                 const std::vector<ast::Statement::Ptr>* body,
                 Symbol_Table captures)
    : parameters_{std::move(parameters)}
    , body_{std::move(body)}
    , captures_{std::move(captures)}
{
}

Value_Ptr eval_or_null(const ast::Statement::Ptr& node, Symbol_Table& syms)
{
    if (auto expr_ptr = dynamic_cast<ast::Expression*>(node.get()))
        return expr_ptr->evaluate(syms);

    node->execute(syms);
    return Value::create(Null{});
}

Value_Ptr Closure::call(const std::vector<Value_Ptr>& args) const
{
    if (args.size() > parameters_.size())
    {
        throw Frost_Error{fmt::format("Closure called with too many arguments. "
                                      "Expected up to {}, but got {}.",
                                      parameters_.size(), args.size())};
    }

    Symbol_Table exec_table(&captures_);
    for (const auto& [arg_name, arg_val] : std::views::zip(
             parameters_, std::views::concat(
                              args, std::views::repeat(Value::create(Null{})))))
    {
        exec_table.define(arg_name, arg_val);
    }

    if (body_->size() == 0)
        return Value::create(Null{});

    for (const ast::Statement::Ptr& node : *body_
                                               | std::views::reverse
                                               | std::views::drop(1)
                                               | std::views::reverse)
    {
        node->execute(exec_table);
    }

    return eval_or_null(body_->back(), exec_table);
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
            << ")";
    }

    os << '\n';

    for (const auto& statement : *body_)
    {
        statement->debug_dump_ast(os);
    }

    return std::move(os).str();
}
