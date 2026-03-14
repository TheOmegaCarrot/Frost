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

    Map(Source_Range source_range, Expression::Ptr structure,
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
