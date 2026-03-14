#ifndef FROST_AST_ARRAY_CONSTRUCTOR_HPP
#define FROST_AST_ARRAY_CONSTRUCTOR_HPP

#include "expression.hpp"

#include <ranges>

namespace frst::ast
{
class Array_Constructor final : public Expression
{
  public:
    using Ptr = std::unique_ptr<Array_Constructor>;

    Array_Constructor() = delete;
    Array_Constructor(const Array_Constructor&) = delete;
    Array_Constructor(Array_Constructor&&) = delete;
    Array_Constructor& operator=(const Array_Constructor&) = delete;
    Array_Constructor& operator=(Array_Constructor&&) = delete;
    ~Array_Constructor() final = default;

    Array_Constructor(Source_Range source_range,
                      std::vector<Expression::Ptr> elems);

    std::string node_label() const final;

    bool data_safe() const final;

  protected:
    [[nodiscard]] Value_Ptr do_evaluate(Evaluation_Context ctx) const final;

    std::generator<Child_Info> children() const final;

  private:
    std::vector<Expression::Ptr> elems_;
};
} // namespace frst::ast

#endif
