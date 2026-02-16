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
    Define(std::string name, Expression::Ptr expr, bool export_def = false);

    Define(const Define&) = delete;
    Define(Define&&) = delete;
    Define& operator=(const Define&) = delete;
    Define& operator=(Define&&) = delete;
    ~Define() final = default;

    std::optional<Map> execute(Symbol_Table& table) const;

    std::generator<Symbol_Action> symbol_sequence() const final;

  protected:
    std::string node_label() const final;

    std::generator<Child_Info> children() const final;

  private:
    std::string name_;
    Expression::Ptr expr_;
    bool export_def_;
};

} // namespace frst::ast

#endif
