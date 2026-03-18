#include <frost/ast/lambda.hpp>
#include <frost/ast/literal.hpp>
#include <frost/ast/utils/block-utils.hpp>
#include <frost/closure.hpp>

#include <fmt/format.h>
#include <fmt/ranges.h>

#include <flat_set>
#include <memory>

namespace frst::ast
{
namespace
{

Value_Ptr promote_if_weak(const Value_Ptr& fn)
{
    if (auto weak_closure =
            std::dynamic_pointer_cast<Weak_Closure>(fn->raw_get<Function>()))
        return Value::create(weak_closure->promote());
    else
        return fn;
}

} // namespace

Lambda::Lambda(Source_Range source_range, std::vector<std::string> params,
               std::vector<Statement::Ptr> body_prefix,
               std::optional<std::string> vararg_param,
               std::optional<std::string> self_name)
    : Expression(source_range)
    , params_{std::move(params)}
    , body_prefix_{std::make_shared<std::vector<Statement::Ptr>>(
          std::move(body_prefix))}
    , vararg_param_{std::move(vararg_param)}
    , self_name_{std::move(self_name)}
{
    std::flat_set<std::string> param_set{std::from_range, params_};

    if (vararg_param_)
        param_set.insert(vararg_param_.value());

    if (self_name_ && param_set.contains(self_name_.value()))
    {
        throw Frost_Unrecoverable_Error{fmt::format(
            "Closure parameter cannot shadow name bound to self ({})",
            self_name_.value())};
    }

    if (const auto expected_param_set_size =
            params_.size() + (vararg_param_.has_value() ? 1 : 0);
        expected_param_set_size != param_set.size())
    {
        throw Frost_Unrecoverable_Error{"Closure has duplicate parameters"};
    }

    if (body_prefix_->size() == 0)
        throw Frost_Unrecoverable_Error("Closure may not have an empty body");

    std::shared_ptr<ast::Statement> last_statement{
        std::move(body_prefix_->back())};
    body_prefix_->pop_back();

    std::shared_ptr<ast::Expression> return_expr =
        std::dynamic_pointer_cast<ast::Expression>(last_statement);
    if (not return_expr)
    {
        throw Frost_Unrecoverable_Error{"A lambda must end in an expression"};
    }
    return_expr_ = std::move(return_expr);

    std::flat_set<std::string> names_defined_so_far{std::from_range, param_set};

    for (const Statement::Symbol_Action& name :
         utils::body_symbol_sequence(*body_prefix_, return_expr_))
    {
        name.visit(Overload{
            [&](const Statement::Definition& defn) {
                if (defn.name == self_name_)
                {
                    throw Frost_Unrecoverable_Error{
                        fmt::format("Closure local definition cannot shadow "
                                    "name bound to self ({})",
                                    self_name_.value())};
                }
                if (param_set.contains(defn.name))
                {
                    throw Frost_Unrecoverable_Error{
                        fmt::format("Closure local definition cannot "
                                    "shadow parameter: {}",
                                    defn.name)};
                }

                names_defined_so_far.insert(defn.name);
            },
            [&](const Statement::Usage& used) {
                if (used.name
                    != self_name_
                    && !names_defined_so_far.contains(used.name))
                {
                    names_to_capture_.insert(used.name);
                }
            },
        });
    }

    closure_define_count_ = names_defined_so_far.size();
}

Value_Ptr Lambda::do_evaluate(Evaluation_Context ctx) const
{
    Symbol_Table captures;
    captures.reserve(names_to_capture_.size());
    for (const std::string& name : names_to_capture_)
    {
        if (not ctx.symbols.has(name))
        {
            throw Frost_Unrecoverable_Error{fmt::format(
                "No definition found for captured symbol: {}", name)};
        }

        const auto& value = ctx.symbols.lookup(name);

        if (value->is<Function>())
        {
            // special handling to promote any closure self-reference to a
            // strong reference
            captures.define(name, promote_if_weak(value));
        }
        else
        {
            captures.define(name, value);
        }
    }

    auto closure = std::make_shared<Closure>(
        params_, body_prefix_, return_expr_, std::move(captures),
        closure_define_count_, vararg_param_, self_name_);

    auto weak_closure = Value::create(
        Function{std::make_shared<Weak_Closure>(std::weak_ptr{closure})});

    if (self_name_)
        closure->inject_capture(self_name_.value(), weak_closure);

    return Value::create(Function{std::move(closure)});
}

std::generator<Statement::Symbol_Action> Lambda::symbol_sequence() const
{

    const auto get_name = [](const auto& action) -> const auto& {
        return action.name;
    };

    std::flat_set<std::string> defns{std::from_range, params_};
    if (self_name_)
        defns.insert(self_name_.value());

    if (vararg_param_)
        defns.insert(vararg_param_.value());

    for (const Statement::Symbol_Action& action :
         utils::body_symbol_sequence(*body_prefix_, return_expr_))
    {
        const auto& name = action.visit(get_name);
        if (std::holds_alternative<Statement::Definition>(action))
            defns.insert(name);
        else if (not defns.contains(name))
            co_yield action;
    }
}

std::string Lambda::do_node_label() const
{
    const bool has_params = !params_.empty() || vararg_param_;
    return fmt::format(
        "Lambda({}{}{}{}{}{})",
        self_name_ ? std::string_view{*self_name_} : std::string_view{},
        self_name_ ? (has_params ? ": " : ":") : "", fmt::join(params_, ", "),
        !params_.empty() && vararg_param_ ? ", " : "",
        vararg_param_ ? "..." : "",
        vararg_param_ ? std::string_view{*vararg_param_} : std::string_view{});
}

std::generator<Statement::Child_Info> Lambda::children() const
{
    for (const auto& statement : *body_prefix_)
        co_yield make_child(statement);
    co_yield make_child(return_expr_);
}

} // namespace frst::ast
