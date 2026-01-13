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
           std::optional<Expression::Ptr> init)
        : structure_{std::move(structure)}
        , operation_{std::move(operation)}
        , init_{std::move(init)}
    {
    }

    [[nodiscard]] Value_Ptr evaluate(const Symbol_Table& syms) const final;

  protected:
    std::generator<Child_Info> children() const final
    {
        co_yield make_child(structure_, "Structure");
        co_yield make_child(operation_, "Operation");
        if (init_)
            co_yield make_child(*init_, "Init");
    }

    std::string node_label() const final
    {
        return "Reduce";
    }

  private:
    Expression::Ptr structure_;
    Expression::Ptr operation_;
    std::optional<Expression::Ptr> init_;
};
} // namespace frst::ast

#endif
