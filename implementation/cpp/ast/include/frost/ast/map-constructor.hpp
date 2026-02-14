#ifndef FROST_AST_MAP_CONSTRUCTOR_HPP
#define FROST_AST_MAP_CONSTRUCTOR_HPP

#include "expression.hpp"

#include <ranges>

namespace frst::ast
{
class Map_Constructor final : public Expression
{
  public:
    using Ptr = std::unique_ptr<Map_Constructor>;
    using KV_Pair = std::pair<Expression::Ptr, Expression::Ptr>;

    Map_Constructor() = delete;
    Map_Constructor(const Map_Constructor&) = delete;
    Map_Constructor(Map_Constructor&&) = delete;
    Map_Constructor& operator=(const Map_Constructor&) = delete;
    Map_Constructor& operator=(Map_Constructor&&) = delete;
    ~Map_Constructor() final = default;

    Map_Constructor(std::vector<KV_Pair> pairs)
        : pairs_{std::move(pairs)}
    {
    }

    [[nodiscard]] Value_Ptr evaluate(const Symbol_Table& syms) const final
    {
        Map acc;

        for (const auto& [key_expr, value_expr] : pairs_)
        {
            auto key = key_expr->evaluate(syms);
            auto value = value_expr->evaluate(syms);
            acc.insert_or_assign(key, value);
        }

        return Value::create(std::move(acc));
    }

  protected:
    std::string node_label() const final
    {
        return "Map_Constructor";
    }

    std::generator<Child_Info> children() const final
    {
        for (const auto& [k, v] : pairs_)
        {
            co_yield make_child(k, "Key");
            co_yield make_child(v, "Value");
        }
    }

  private:
    std::vector<KV_Pair> pairs_;
};
} // namespace frst::ast

#endif
