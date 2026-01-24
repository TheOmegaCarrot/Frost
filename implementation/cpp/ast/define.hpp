#ifndef FROST_AST_DEFINE_HPP
#define FROST_AST_DEFINE_HPP

#include "expression.hpp"
#include "statement.hpp"

#include <fmt/format.h>

namespace frst::ast
{

class Define final : public Statement
{
  public:
    using Ptr = std::unique_ptr<Define>;

    Define() = delete;
    Define(std::string name, Expression::Ptr expr)
        : name_{std::move(name)}
        , expr_{std::move(expr)}
    {
        if (name_ == "_")
            throw Frost_Unrecoverable_Error{"\"_\" is not a valid identifier"};
    }

    Define(const Define&) = delete;
    Define(Define&&) = delete;
    Define& operator=(const Define&) = delete;
    Define& operator=(Define&&) = delete;
    ~Define() final = default;

    void execute(Symbol_Table& table) const
    {
        table.define(name_, expr_->evaluate(table));
    }

    std::generator<Symbol_Action> symbol_sequence() const final
    {
        co_yield std::ranges::elements_of(expr_->symbol_sequence());
        co_yield Definition{name_};
    }

  protected:
    std::string node_label() const final
    {
        return fmt::format("Define({})", name_);
    }

    std::generator<Child_Info> children() const final
    {
        co_yield make_child(expr_);
    }

  private:
    std::string name_;
    Expression::Ptr expr_;
};

} // namespace frst::ast

#endif
