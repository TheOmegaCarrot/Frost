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
    Define(std::string name, std::unique_ptr<Expression> expr)
        : name_{std::move(name)}
        , expr_{std::move(expr)}
    {
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

  protected:
    std::string node_label() const final
    {
        return fmt::format("Define({})", name_);
    }

    std::generator<Child_Info> children() const final
    {
        co_yield make_child(expr_, "Assignment_Expr");
    }

  private:
    std::string name_;
    std::unique_ptr<Expression> expr_;
};

} // namespace frst::ast

#endif
