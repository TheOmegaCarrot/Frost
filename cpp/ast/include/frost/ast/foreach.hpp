#ifndef FROST_AST_FOREACH_HPP
#define FROST_AST_FOREACH_HPP

#include "expression.hpp"
#include "frost/value.hpp"

namespace frst::ast
{

class Foreach final : public Expression
{
  public:
    using Ptr = std::unique_ptr<Foreach>;

    Foreach() = delete;
    Foreach(const Foreach&) = delete;
    Foreach(Foreach&&) = delete;
    Foreach& operator=(const Foreach&) = delete;
    Foreach& operator=(Foreach&&) = delete;
    ~Foreach() final = default;

    Foreach(Expression::Ptr structure, Expression::Ptr operation);

    std::string node_label() const final;

  protected:
    [[nodiscard]] Value_Ptr do_evaluate(const Symbol_Table& syms) const final;

    std::generator<Child_Info> children() const final;

  private:
    Expression::Ptr structure_;
    Expression::Ptr operation_;
};

} // namespace frst::ast

#endif
