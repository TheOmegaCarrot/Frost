#ifndef FROST_AST_LAMBDA_HPP
#define FROST_AST_LAMBDA_HPP

#include <frost/ast.hpp>
#include <frost/closure.hpp>

#include <fmt/format.h>
#include <fmt/ranges.h>

/*
namespace frst::ast
{
class Lambda final : public Expression
{
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
    }

    [[nodiscard]] Value_Ptr evaluate(const Symbol_Table& syms) const final
    {
        return Value::create(
            Function{std::make_shared<Closure>(params_, &body_, syms)});
    }

    std::generator<Symbol_Action> symbol_sequence() const final
    {
        co_return;
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
    std::vector<Statement::Ptr> body_;
};
} // namespace frst::ast

*/
#endif
