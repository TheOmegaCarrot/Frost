#ifndef FROST_AST_LAMBDA_HPP
#define FROST_AST_LAMBDA_HPP

#include <frost/ast.hpp>
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
} // namespace

class Lambda final : public Expression
{
  public:
    using Ptr = std::unique_ptr<Lambda>;

    Lambda() = delete;
    Lambda(const Lambda&) = delete;
    Lambda(Lambda&&) = delete;
    Lambda& operator=(const Lambda&) = delete;
    Lambda& operator=(Lambda&&) = delete;
    ~Lambda() final = default;

    Lambda(std::vector<std::string> params, std::vector<Statement::Ptr> body,
           std::optional<std::string> vararg_param = {})
        : params_{std::move(params)}
        , body_{std::make_shared<std::vector<Statement::Ptr>>(std::move(body))}
        , vararg_param_{std::move(vararg_param)}
    {
        auto param_set =
            params_ | std::ranges::to<std::flat_set<std::string>>();

        if (vararg_param_)
            param_set.insert(vararg_param_.value());

        if (const auto expected_param_set_size =
                params_.size() + (vararg_param_.has_value() ? 1 : 0);
            expected_param_set_size != param_set.size())
        {
            throw Frost_Internal_Error{"Closure has duplicate parameters"};
        }

        std::flat_set<std::string> names_defined_so_far{std::from_range,
                                                        param_set};

        for (const Statement::Symbol_Action& name :
             *body_
                 | std::views::transform(&node_to_sym_seq)
                 | std::views::join)
        {
            name.visit(Overload{
                [&](const Statement::Definition& defn) {
                    if (param_set.contains(defn.name))
                    {
                        throw Frost_Internal_Error{
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
    }

    [[nodiscard]] Value_Ptr evaluate(const Symbol_Table& syms) const final
    {
        Symbol_Table captures;
        for (const std::string& name : names_to_capture_)
        {
            if (not syms.has(name))
            {
                throw Frost_Internal_Error{fmt::format(
                    "No definition found for captured symbol: {}", name)};
            }

            captures.define(name, syms.lookup(name));
        }

        auto closure = std::make_shared<Closure>(
            params_, body_, std::move(captures), vararg_param_);

        auto weak_closure = Value::create(
            Function{std::make_shared<Weak_Closure>(std::weak_ptr{closure})});

        closure->inject_capture("self", weak_closure);

        return Value::create(Function{std::move(closure)});
    }

    std::generator<Symbol_Action> symbol_sequence() const final
    {
        const auto get_name = [](const auto& action) {
            return action.name;
        };

        std::flat_set<std::string> defns{std::from_range, params_};

        if (vararg_param_)
            defns.insert(vararg_param_.value());

        for (const Statement::Symbol_Action& action :
             *body_
                 | std::views::transform(&node_to_sym_seq)
                 | std::views::join)
        {
            const auto name = action.visit(get_name);
            if (std::holds_alternative<Statement::Definition>(action))
                defns.insert(name);
            else if (not defns.contains(name))
                co_yield action;
        }
    }

  protected:
    std::string node_label() const final
    {
        if (vararg_param_)
        {
            return fmt::format("Lambda({}, ...{})", fmt::join(params_, ", "),
                               vararg_param_.value());
        }
        else
        {
            return fmt::format("Lambda({})", fmt::join(params_, ", "));
        }
    }

    std::generator<Child_Info> children() const final
    {
        for (const auto& statement : *body_)
            co_yield make_child(statement);
    }

  private:
    std::vector<std::string> params_;
    std::flat_set<std::string> names_to_capture_;
    std::shared_ptr<std::vector<Statement::Ptr>> body_;
    std::optional<std::string> vararg_param_;
};
} // namespace frst::ast

#endif
