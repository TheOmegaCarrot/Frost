#ifndef FROST_AST_MAP_HPP
#define FROST_AST_MAP_HPP

#include "expression.hpp"

namespace frst::ast
{

class Map final : public Expression
{

  public:
    using Ptr = std::unique_ptr<Map>;

    Map() = delete;
    Map(const Map&) = delete;
    Map(Map&&) = delete;
    Map& operator=(const Map&) = delete;
    Map& operator=(Map&&) = delete;
    ~Map() final = default;

    Map(Expression::Ptr structure, Expression::Ptr operation);

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
