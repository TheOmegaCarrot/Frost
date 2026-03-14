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

    Literal(Source_Range source_range, Value_Ptr value);

    Literal() = delete;
    Literal(const Literal&) = delete;
    Literal(Literal&&) = delete;
    Literal& operator=(const Literal&) = delete;
    Literal& operator=(Literal&&) = delete;
    ~Literal() final = default;

    std::string node_label() const final;

    bool data_safe() const final;

  protected:
    [[nodiscard]] Value_Ptr do_evaluate(Evaluation_Context ctx) const final;

  private:
    Value_Ptr value_;
};
} // namespace frst::ast

#endif
