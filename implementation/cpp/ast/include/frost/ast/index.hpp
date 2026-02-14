#ifndef FROST_AST_INDEX_HPP
#define FROST_AST_INDEX_HPP

#include "expression.hpp"

#include <frost/value.hpp>

#include <fmt/format.h>

namespace frst::ast
{

class Index final : public Expression
{
  public:
    using Ptr = std::unique_ptr<Index>;

    Index(Expression::Ptr structure, Expression::Ptr index)
        : structure_{std::move(structure)}
        , index_{std::move(index)}
    {
    }

    Index() = delete;
    Index(const Index&) = delete;
    Index(Index&&) = delete;
    Index& operator=(const Index&) = delete;
    Index& operator=(Index&&) = delete;
    ~Index() final = default;

    [[nodiscard]] Value_Ptr evaluate(const Symbol_Table& syms) const final;

  protected:
    std::string node_label() const final
    {
        return "Index_Expression";
    }

    std::generator<Child_Info> children() const final
    {
        co_yield make_child(structure_, "Structure");
        co_yield make_child(index_, "Index");
    }

  private:
    Expression::Ptr structure_;
    Expression::Ptr index_;
};
} // namespace frst::ast

#endif
