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

    Filter(Expression::Ptr structure, Expression::Ptr operation)
        : structure_{std::move(structure)}
        , operation_{std::move(operation)}
    {
    }

    [[nodiscard]] Value_Ptr evaluate(const Symbol_Table& syms) const final;

  protected:
    std::generator<Child_Info> children() const final
    {
        co_yield make_child(structure_, "Structure");
        co_yield make_child(operation_, "Operation");
    }

    std::string node_label() const final
    {
        return "Filter";
    }

  private:
    Expression::Ptr structure_;
    Expression::Ptr operation_;
};

} // namespace frst::ast

#endif
