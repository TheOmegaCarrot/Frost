#include <frost/ast.hpp>
#include <frost/closure.hpp>
#include <frost/symbol-table.hpp>

#include <fmt/format.h>

#include <algorithm>
#include <sstream>

using namespace frst;

Closure::Closure(std::vector<std::string> parameters,
                 std::shared_ptr<std::vector<ast::Statement::Ptr>> body,
                 Symbol_Table captures,
                 std::optional<std::string> vararg_parameter)
    : parameters_{std::move(parameters)}
    , body_{std::move(body)}
    , captures_{std::move(captures)}
    , vararg_parameter_{std::move(vararg_parameter)}
{
    // Assumed: all params in parameters_ and vararg_parameter_ (if present) are
    // all unique. No duplicates exist.
    // This must be checked by the Lambda AST node.
}

Value_Ptr eval_or_null(const ast::Statement::Ptr& node, Symbol_Table& syms)
{
    if (auto expr_ptr = dynamic_cast<ast::Expression*>(node.get()))
        return expr_ptr->evaluate(syms);

    node->execute(syms);
    return Value::null();
}

Value_Ptr Closure::call(std::span<const Value_Ptr> args) const
{
    if (!vararg_parameter_ && args.size() > parameters_.size())
    {
        throw Frost_User_Error{
            fmt::format("Closure called with too many arguments. "
                        "Expected up to {}, but got {}.",
                        parameters_.size(), args.size())};
    }

    Symbol_Table exec_table(&captures_);
    for (const auto& [arg_name, arg_val] : std::views::zip(
             parameters_,
             std::views::concat(args, std::views::repeat(Value::null()))))
    {
        exec_table.define(arg_name, arg_val);
    }

    if (vararg_parameter_)
    {
        exec_table.define(vararg_parameter_.value(),
                          Value::create(args
                                        | std::views::drop(parameters_.size())
                                        | std::ranges::to<Array>()));
    }

    if (body_->size() == 0)
        return Value::null();

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

    if (captures_.debug_table().size() > 1)
    {
        os
            << " (capturing: "
            << (captures_.debug_table()
                | std::views::keys
                | std::views::filter([](const auto& key) {
                      return key != "self";
                  })
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
