#ifndef FROST_AST_DO_HPP
#define FROST_AST_DO_HPP

#include "expression.hpp"

namespace frst::ast
{

class Do_Block final : public Expression
{
  public:
    using Ptr = std::unique_ptr<Do_Block>;

    Do_Block(Source_Range source_range, std::vector<ast::Statement::Ptr> body);

    Do_Block() = delete;
    Do_Block(const Do_Block&) = delete;
    Do_Block(Do_Block&&) = delete;
    Do_Block& operator=(const Do_Block&) = delete;
    Do_Block& operator=(Do_Block&&) = delete;
    ~Do_Block() final = default;

    std::generator<Symbol_Action> symbol_sequence() const final;

  protected:
    std::string do_node_label() const final;

    [[nodiscard]] Value_Ptr do_evaluate(Evaluation_Context ctx) const final;

    std::generator<Child_Info> children() const final;

  private:
    std::vector<ast::Statement::Ptr> body_prefix_;
    ast::Expression::Ptr value_expr_;
};

} // namespace frst::ast

#endif
