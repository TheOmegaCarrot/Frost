#ifndef FROST_AST_FILTER_HPP
#define FROST_AST_FILTER_HPP

#include "expression.hpp"

namespace frst::ast
{

class Filter final : public Expression
{

  public:
    using Ptr = std::unique_ptr<Filter>;

    Filter() = delete;
    Filter(const Filter&) = delete;
    Filter(Filter&&) = delete;
    Filter& operator=(const Filter&) = delete;
    Filter& operator=(Filter&&) = delete;
    ~Filter() final = default;

    Filter(Expression::Ptr structure, Expression::Ptr operation);

    std::string node_label() const final;

  protected:
    [[nodiscard]] Value_Ptr do_evaluate(Evaluation_Context ctx) const final;

    std::generator<Child_Info> children() const final;

  private:
    Expression::Ptr structure_;
    Expression::Ptr operation_;
};

} // namespace frst::ast

#endif
