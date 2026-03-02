#ifndef FROST_AST_IF_HPP
#define FROST_AST_IF_HPP

#include "expression.hpp"

namespace frst::ast
{
class If final : public Expression
{
  public:
    using Ptr = std::unique_ptr<If>;

    If(Expression::Ptr condition, Expression::Ptr consequent,
       std::optional<Expression::Ptr> alternate = std::nullopt);

    If() = delete;
    If(const If&) = delete;
    If(If&&) = delete;
    If& operator=(const If&) = delete;
    If& operator=(If&&) = delete;
    ~If() final = default;

    [[nodiscard]] Value_Ptr evaluate(const Symbol_Table& syms) const final;

  protected:
    std::string node_label() const final;

    std::generator<Child_Info> children() const final;

  private:
    Expression::Ptr condition_;
    Expression::Ptr consequent_;
    std::optional<Expression::Ptr> alternate_;

    // elseif is handled by using another entire if as the alternate
    //
    // ```
    // if foo: 1
    // elif bar: 2
    // else: 3
    // ```
    // The above is effectively syntax sugar for the equivalent syntax below:
    // (whitespace shown is not relevant to the grammer,
    //      but makes the structure more clear)
    // ```
    // if foo: 1
    // else: if bar: 2
    //       else: 3
    // ```
};
} // namespace frst::ast

#endif
