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
       std::optional<Expression::Ptr> alternate = std::nullopt)
        : condition_{std::move(condition)}
        , consequent_{std::move(consequent)}
        , alternate_{std::move(alternate)}
    {
    }

    If() = delete;
    If(const If&) = delete;
    If(If&&) = delete;
    If& operator=(const If&) = delete;
    If& operator=(If&&) = delete;
    ~If() final = default;

    [[nodiscard]] Value_Ptr evaluate(const Symbol_Table& syms) const final
    {
        if (condition_->evaluate(syms)->as<Bool>().value())
            return consequent_->evaluate(syms);
        else if (alternate_.has_value())
            return (*alternate_)->evaluate(syms);

        // If an if-expression has no alternate branch,
        // and the condition is false, then evaluate to null
        return Value::create(Null{});
    }

  protected:
    std::string node_label() const final
    {
        return "If";
    }

    std::generator<Child_Info> children() const final
    {
        co_yield make_child(condition_, "Condition");
        co_yield make_child(consequent_, "Consequent");
        if (alternate_)
            co_yield make_child(*alternate_, "Alternate");
    }

  private:
    Expression::Ptr condition_;
    Expression::Ptr consequent_;
    std::optional<Expression::Ptr> alternate_;

    // elseif is handled by using another entire if as the alternate
    //
    // ```
    // if foo: 1
    // elseif bar: 2
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
