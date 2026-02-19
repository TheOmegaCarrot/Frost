#include <frost/ast.hpp>
#include <frost/ast/lambda.hpp>
#include <frost/closure.hpp>

#include <fmt/format.h>
#include <fmt/ranges.h>

#include <flat_set>
#include <memory>

namespace frst::ast
{
namespace
{

std::generator<Statement::Symbol_Action> node_to_sym_seq(
    const frst::ast::Statement::Ptr& node)
{
    return node->symbol_sequence();
}

Value_Ptr promote_if_weak(const Value_Ptr& fn)
{
    if (auto weak_closure =
            std::dynamic_pointer_cast<Weak_Closure>(fn->raw_get<Function>()))
        return Value::create(weak_closure->promote());
    else
        return fn;
}

} // namespace

Lambda::Lambda(std::vector<std::string> params,
               std::vector<Statement::Ptr> body,
               std::optional<std::string> vararg_param)
    : params_{std::move(params)}
    , body_{std::make_shared<std::vector<Statement::Ptr>>(std::move(body))}
    , vararg_param_{std::move(vararg_param)}
{
    std::flat_set<std::string> param_set{std::from_range, params_};

    if (vararg_param_)
        param_set.insert(vararg_param_.value());

    if (param_set.contains("self"))
    {
        throw Frost_Unrecoverable_Error{
            "Closure parameter cannot be named self"};
    }

    if (const auto expected_param_set_size =
            params_.size() + (vararg_param_.has_value() ? 1 : 0);
        expected_param_set_size != param_set.size())
    {
        throw Frost_Unrecoverable_Error{"Closure has duplicate parameters"};
    }

    std::flat_set<std::string> names_defined_so_far{std::from_range, param_set};

    for (const Statement::Symbol_Action& name :
         *body_ | std::views::transform(&node_to_sym_seq) | std::views::join)
    {
        name.visit(Overload{
            [&](const Statement::Definition& defn) {
                if (defn.name == "self")
                {
                    throw Frost_Unrecoverable_Error{
                        "Closure local definition cannot shadow self"};
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
                    != "self"
                    && !names_defined_so_far.contains(used.name))
                {
                    names_to_capture_.insert(used.name);
                }
            },
        });
    }

    closure_define_count_ = names_defined_so_far.size();
}

[[nodiscard]] Value_Ptr Lambda::evaluate(const Symbol_Table& syms) const
{
    Symbol_Table captures;
    captures.reserve(names_to_capture_.size());
    for (const std::string& name : names_to_capture_)
    {
        if (not syms.has(name))
        {
            throw Frost_Unrecoverable_Error{fmt::format(
                "No definition found for captured symbol: {}", name)};
        }

        const auto& value = syms.lookup(name);

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

    auto closure =
        std::make_shared<Closure>(params_, body_, std::move(captures),
                                  closure_define_count_, vararg_param_);

    auto weak_closure = Value::create(
        Function{std::make_shared<Weak_Closure>(std::weak_ptr{closure})});

    closure->inject_capture("self", weak_closure);

    return Value::create(Function{std::move(closure)});
}

std::generator<Statement::Symbol_Action> Lambda::symbol_sequence() const
{

    const auto get_name = [](const auto& action) {
        return action.name;
    };

    std::flat_set<std::string> defns{std::from_range, params_};
    defns.insert("self");

    if (vararg_param_)
        defns.insert(vararg_param_.value());

    for (const Statement::Symbol_Action& action :
         *body_ | std::views::transform(&node_to_sym_seq) | std::views::join)
    {
        const auto name = action.visit(get_name);
        if (std::holds_alternative<Statement::Definition>(action))
            defns.insert(name);
        else if (not defns.contains(name))
            co_yield action;
    }
}

std::string Lambda::node_label() const
{
    if (vararg_param_ && params_.size() != 0)
    {
        return fmt::format("Lambda({}, ...{})", fmt::join(params_, ", "),
                           vararg_param_.value());
    }
    else if (vararg_param_ && params_.size() == 0)
    {
        return fmt::format("Lambda(...{})", vararg_param_.value());
    }
    else
    {
        return fmt::format("Lambda({})", fmt::join(params_, ", "));
    }
}

std::generator<Statement::Child_Info> Lambda::children() const
{
    for (const auto& statement : *body_)
        co_yield make_child(statement);
}

} // namespace frst::ast
