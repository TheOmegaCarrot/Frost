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

    Array_Constructor(std::vector<Expression::Ptr> elems)
        : elems_{std::move(elems)}
    {
    }

    [[nodiscard]] Value_Ptr evaluate(const Symbol_Table& syms) const final
    {
        return Value::create(elems_
                             | std::views::transform([&](const auto& each) {
                                   return each->evaluate(syms);
                               })
                             | std::ranges::to<std::vector>());
    }

  protected:
    std::string node_label() const final
    {
        return "Array";
    }

    std::generator<Child_Info> children() const final
    {
        for (const auto& elem : elems_)
            co_yield make_child(elem);
    }

  private:
    std::vector<Expression::Ptr> elems_;
};
} // namespace frst::ast

#endif
