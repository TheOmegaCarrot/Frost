#ifndef FROST_AST_LITERAL_HPP
#define FROST_AST_LITERAL_HPP

#include "expression.hpp"

namespace frst::ast
{
//! @brief A literal is exactly a primative literal
//          (null, int, float, bool, string)
//          This node type does NOT include arrays, maps, or closures
class Literal final : public Expression
{
#pragma message("TODO: Test Literal")
  public:
    using Ptr = std::unique_ptr<Literal>;

    Literal() = delete;

    Literal(Value_Ptr value)
        : value_{std::move(value)}
    {
    }

    Literal(const Literal&) = delete;
    Literal(Literal&&) = delete;
    Literal& operator=(const Literal&) = delete;
    Literal& operator=(Literal&&) = delete;
    ~Literal() final = default;

    [[nodiscard]] virtual Value_Ptr evaluate(const Symbol_Table&) const final
    {
        return value_;
    }

  protected:
    std::string node_label() const final
    {
        return "Literal(<TODO: TOSTRING VALUE HERE>)";
    }

  private:
    Value_Ptr value_;
};
} // namespace frst::ast

#endif
