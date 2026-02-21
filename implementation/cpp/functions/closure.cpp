#include <frost/ast.hpp>
#include <frost/closure.hpp>
#include <frost/symbol-table.hpp>

#include <fmt/format.h>

#include <algorithm>
#include <sstream>

using namespace frst;

Closure::Closure(std::vector<std::string> parameters,
                 std::shared_ptr<std::vector<ast::Statement::Ptr>> body_prefix,
                 std::shared_ptr<ast::Expression> return_expr,
                 Symbol_Table captures, std::size_t define_count,
                 std::optional<std::string> vararg_parameter)
    : parameters_{std::move(parameters)}
    , body_prefix_{std::move(body_prefix)}
    , return_expr_{std::move(return_expr)}
    , captures_{std::move(captures)}
    , vararg_parameter_{std::move(vararg_parameter)}
    , define_count_{define_count}
{
    // Assumed: all params in parameters_ and vararg_parameter_ (if present) are
    // all unique. No duplicates exist.
    // This must be checked by the Lambda AST node.
}

const Symbol_Table& Closure::debug_capture_table() const
{
    return captures_;
}

void Closure::inject_capture(const std::string& name, Value_Ptr value)
{
    captures_.define(name, value);
}

Value_Ptr Closure::call(std::span<const Value_Ptr> args) const
{
    if (!vararg_parameter_ && args.size() > parameters_.size())
    {
        throw Frost_Recoverable_Error{
            fmt::format("Closure called with too many arguments. "
                        "Expected up to {}, but got {}.",
                        parameters_.size(), args.size())};
    }

    Symbol_Table exec_table(&captures_);
    exec_table.reserve(define_count_);
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

    for (const ast::Statement::Ptr& node : *body_prefix_)
    {
        node->execute(exec_table);
    }

    return return_expr_->evaluate(exec_table);
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

    for (const auto& statement : *body_prefix_)
    {
        statement->debug_dump_ast(os);
    }

    return std::move(os).str();
}

Weak_Closure::Weak_Closure(std::weak_ptr<Closure> closure)
    : closure_{closure}
{
}

Value_Ptr Weak_Closure::call(std::span<const Value_Ptr> args) const
{
    if (auto closure = closure_.lock())
        return closure->call(args);

    throw Frost_Internal_Error{"Closure self-reference expired"};
}

Function Weak_Closure::promote() const
{
    if (auto closure = closure_.lock())
        return std::static_pointer_cast<Callable>(closure);

    throw Frost_Internal_Error{"Failed to promote closure self-reference"};
}

std::string Weak_Closure::debug_dump() const
{
    return "<closure self-reference>";
}
