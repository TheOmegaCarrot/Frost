#ifndef FROST_AST_DEFINE_HPP
#define FROST_AST_DEFINE_HPP

#include "destructure.hpp"
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
    Define(const Source_Range& source_range, Destructure::Ptr destructure,
           Expression::Ptr expr);

    Define(const Define&) = delete;
    Define(Define&&) = delete;
    Define& operator=(const Define&) = delete;
    Define& operator=(Define&&) = delete;
    ~Define() final = default;

    std::generator<Symbol_Action> symbol_sequence() const final;
    std::generator<Child_Info> children() const final;

  protected:
    std::string do_node_label() const final;

    void do_execute(Execution_Context& ctx) const final;

  private:
    Destructure::Ptr destructure_;
    Expression::Ptr expr_;
};

} // namespace frst::ast

#endif
