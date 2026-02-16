#ifndef FROST_AST_LITERAL_HPP
#define FROST_AST_LITERAL_HPP

#include "expression.hpp"

#include <fmt/format.h>

namespace frst::ast
{
//! @brief A literal is exactly a primative literal
//          (null, int, float, bool, string)
//          This node type does NOT include arrays, maps, or closures
class Literal final : public Expression
{
  public:
    using Ptr = std::unique_ptr<Literal>;

    Literal(Value_Ptr value);

    Literal() = delete;
    Literal(const Literal&) = delete;
    Literal(Literal&&) = delete;
    Literal& operator=(const Literal&) = delete;
    Literal& operator=(Literal&&) = delete;
    ~Literal() final = default;

    [[nodiscard]] Value_Ptr evaluate(const Symbol_Table&) const final;

  protected:
    std::string node_label() const final;

  private:
    Value_Ptr value_;
};
} // namespace frst::ast

#endif
