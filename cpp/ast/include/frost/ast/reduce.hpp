#ifndef FROST_AST_REDUCE_HPP
#define FROST_AST_REDUCE_HPP

#include "expression.hpp"

namespace frst::ast
{

class Reduce final : public Expression
{
  public:
    using Ptr = std::unique_ptr<Reduce>;

    Reduce() = delete;
    Reduce(const Reduce&) = delete;
    Reduce(Reduce&&) = delete;
    Reduce& operator=(const Reduce&) = delete;
    Reduce& operator=(Reduce&&) = delete;
    ~Reduce() final = default;

    Reduce(Expression::Ptr structure, Expression::Ptr operation,
           std::optional<Expression::Ptr> init);

    [[nodiscard]] Value_Ptr evaluate(const Symbol_Table& syms) const final;

  protected:
    std::generator<Child_Info> children() const final;

    std::string node_label() const final;

  private:
    Expression::Ptr structure_;
    Expression::Ptr operation_;
    std::optional<Expression::Ptr> init_;
};
} // namespace frst::ast

#endif
