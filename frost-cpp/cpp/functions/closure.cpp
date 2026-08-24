#include <frost/ast.hpp>
#include <frost/closure.hpp>
#include <frost/symbol-table.hpp>

#include <fmt/format.h>

#include <sstream>

using namespace frst;

Closure::Closure(std::vector<std::string> parameters,
                 std::shared_ptr<std::vector<ast::Statement::Ptr>> body_prefix,
                 std::shared_ptr<ast::Expression> return_expr,
                 Symbol_Table captures, std::size_t define_count,
                 std::optional<std::string> vararg_parameter,
                 std::optional<std::string> self_name)
    : parameters_{std::move(parameters)}
    , body_prefix_{std::move(body_prefix)}
    , return_expr_{std::move(return_expr)}
    , captures_{std::move(captures)}
    , vararg_parameter_{std::move(vararg_parameter)}
    , self_name_{std::move(self_name)}
    , define_count_{define_count}
{
    // Assumed: all params in parameters_ and vararg_parameter_ (if present) are
    // all unique. No duplicates exist.
    // This must be checked by the Lambda AST node.
}

std::shared_ptr<Closure> Closure::create(
    std::vector<std::string> parameters,
    std::shared_ptr<std::vector<ast::Statement::Ptr>> body,
    std::shared_ptr<ast::Expression> return_expr, Symbol_Table captures,
    std::size_t define_count, std::optional<std::string> vararg_parameter,
    std::optional<std::string> self_name)
{
    return std::make_shared<Closure>(
        std::move(parameters), std::move(body), std::move(return_expr),
        std::move(captures), define_count, std::move(vararg_parameter),
        std::move(self_name));
}

const Symbol_Table& Closure::debug_capture_table() const
{
    return captures_;
}

Function Closure::self_function() const
{
    return std::static_pointer_cast<const Callable>(shared_from_this());
}

Value_Ptr Closure::call(std::span<const Value_Ptr> args) const
{
    if (!vararg_parameter_ && args.size() != parameters_.size())
    {
        throw Frost_Recoverable_Error{
            fmt::format("Closure called with wrong number of arguments. "
                        "Expected {}, but got {}.",
                        parameters_.size(), args.size())};
    }

    if (vararg_parameter_ && args.size() < parameters_.size())
    {
        throw Frost_Recoverable_Error{
            fmt::format("Closure called with wrong number of arguments. "
                        "Expected at least {}, but got {}.",
                        parameters_.size(), args.size())};
    }

    Symbol_Table scope_table(&captures_);
    Execution_Context scope_ctx{.symbols = scope_table};
    scope_ctx.symbols.reserve(define_count_ + (self_name_ ? 1 : 0));
    for (const auto& [arg_name, arg_val] : std::views::zip(parameters_, args))
    {
        scope_ctx.symbols.define(arg_name, arg_val);
    }

    if (vararg_parameter_)
    {
        scope_ctx.symbols.define(
            vararg_parameter_.value(),
            Value::create(args
                          | std::views::drop(parameters_.size())
                          | std::ranges::to<Array>()));
    }

    if (self_name_)
        scope_ctx.symbols.define(self_name_.value(),
                                 Value::create(self_function()));

    if (parameters_.size() != 0 && parameters_.front() == "$1")
        scope_ctx.symbols.define("$", args.front());

    for (const ast::Statement::Ptr& node : *body_prefix_)
    {
        node->execute(scope_ctx);
    }

    return return_expr_->evaluate(scope_ctx.as_eval());
}

std::string Closure::name() const
{
    if (self_name_)
        return self_name_.value();
    return "<anonymous>";
}

std::string Closure::debug_dump() const
{
    std::ostringstream os;
    os << "<Closure>";

    if (not captures_.empty())
    {
        os
            << " (capturing: "
            << (captures_.names()
                | std::views::join_with(',')
                | std::ranges::to<std::string>())
            << ")";
    }

    os << '\n';

    for (const auto& statement : *body_prefix_)
    {
        statement->debug_dump_ast(os);
    }

    return_expr_->debug_dump_ast(os);

    return std::move(os).str();
}
