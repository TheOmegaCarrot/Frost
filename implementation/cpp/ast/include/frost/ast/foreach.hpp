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

    [[nodiscard]] Value_Ptr evaluate(const Symbol_Table& syms) const final;

  protected:
    std::generator<Child_Info> children() const final;

    std::string node_label() const final;

  private:
    Expression::Ptr structure_;
    Expression::Ptr operation_;
};

} // namespace frst::ast

#endif
