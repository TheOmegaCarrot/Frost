#ifndef FROST_AST_LAMBDA_HPP
#define FROST_AST_LAMBDA_HPP

#include <frost/ast.hpp>
#include <frost/closure.hpp>

#include <fmt/format.h>
#include <fmt/ranges.h>

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

    Lambda(std::vector<std::string> params, std::vector<Statement::Ptr> body)
        : params_{std::move(params)}
        , body_{std::move(body)}
    {
        const auto param_set = params_ | std::ranges::to<std::flat_set>();
        if (params_.size() != param_set.size())
        {
            throw Frost_Internal_Error{"Closure has duplicate parameters"};
        }

        std::flat_set<std::string> names_defined_so_far{std::from_range,
                                                        params_};
        for (const Statement::Symbol_Action& name :
             body_ | std::views::transform(&node_to_sym_seq) | std::views::join)
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
                    if (!names_defined_so_far.contains(used.name))
                        names_to_capture_.insert(used.name);
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

        return Value::create(Function{
            std::make_shared<Closure>(params_, &body_, std::move(captures))});
    }

    std::generator<Symbol_Action> symbol_sequence() const final
    {
        const auto get_name = [](const auto& action) {
            return action.name;
        };

        std::flat_set<std::string> defns{std::from_range, params_};
        for (const Statement::Symbol_Action& action :
             body_ | std::views::transform(&node_to_sym_seq) | std::views::join)
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
        return fmt::format("Lambda({})", fmt::join(params_, ", "));
    }

    std::generator<Child_Info> children() const final
    {
        for (const auto& statement : body_)
            co_yield make_child(statement);
    }

  private:
    std::vector<std::string> params_;
    std::flat_set<std::string> names_to_capture_;
    std::vector<Statement::Ptr> body_;
};
} // namespace frst::ast

#endif
