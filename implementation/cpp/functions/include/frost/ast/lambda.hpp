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
           std::optional<std::string> vararg_param = {});

    [[nodiscard]] Value_Ptr evaluate(const Symbol_Table& syms) const final;

    std::generator<Symbol_Action> symbol_sequence() const final;

  protected:
    std::string node_label() const final;

    std::generator<Child_Info> children() const final;

  private:
    std::vector<std::string> params_;
    std::flat_set<std::string> names_to_capture_;
    std::shared_ptr<std::vector<Statement::Ptr>> body_;
    std::optional<std::string> vararg_param_;
};
} // namespace frst::ast

#endif
