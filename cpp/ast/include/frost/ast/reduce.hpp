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

    Reduce(Source_Range source_range, Expression::Ptr structure,
           Expression::Ptr operation, std::optional<Expression::Ptr> init);

  protected:
    std::string do_node_label() const final;

    [[nodiscard]] Value_Ptr do_evaluate(Evaluation_Context ctx) const final;

    std::generator<Child_Info> children() const final;

  private:
    Expression::Ptr structure_;
    Expression::Ptr operation_;
    std::optional<Expression::Ptr> init_;
};
} // namespace frst::ast

#endif
