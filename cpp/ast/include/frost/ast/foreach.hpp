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

    Foreach(Source_Range source_range, Expression::Ptr structure,
            Expression::Ptr operation);

  protected:
    std::string do_node_label() const final;

    [[nodiscard]] Value_Ptr do_evaluate(Evaluation_Context ctx) const final;

    std::generator<Child_Info> children() const final;

  private:
    Expression::Ptr structure_;
    Expression::Ptr operation_;
};

} // namespace frst::ast

#endif
